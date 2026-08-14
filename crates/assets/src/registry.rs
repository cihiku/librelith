use core::{
    any::{Any, TypeId, type_name},
    borrow::Borrow,
    error::Error,
    marker::PhantomData,
    ops::Deref,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    Name, Property, StableId, StateId,
    facet::{Column, Facet, Storage},
    id::Id,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError<S> {
    DuplicateStableId(S),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError<S> {
    pub(crate) errors: Vec<BuildErrorKind<S>>,
}

impl<S> BuildError<S> {
    pub fn errors(&self) -> &[BuildErrorKind<S>] {
        &self.errors
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildErrorKind<S> {
    MissingComponent {
        entry: S,
        component: &'static str,
    },
    DuplicateComponent {
        entry: S,
        component: &'static str,
    },
    PinOutOfRange {
        entry: S,
        slot: u32,
        len: u32,
    },
    PinConflict {
        first: S,
        second: S,
        slot: u32,
    },
    DoublePin {
        entry: S,
        first: u32,
        second: u32,
    },
    DuplicateColumn {
        component: &'static str,
    },
    InsertOutOfRange {
        component: &'static str,
        id: u32,
        len: u32,
    },
    CapExceeded {
        len: u32,
        cap: u32,
    },
    DuplicateProperty {
        entry: S,
        property: &'static str,
    },
    TooManyStates {
        entry: S,
    },
    TooManyStatesTotal {
        entry: S,
    },
}

impl<S: core::fmt::Display> core::fmt::Display for RegistryError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DuplicateStableId(stable_id) => write!(f, "{stable_id} already in registry"),
        }
    }
}

impl<S: core::fmt::Display> core::fmt::Display for BuildError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let n = self.errors.len();
        write!(f, "{} build error{}:", n, if n == 1 { "" } else { "s" })?;
        for (i, error) in self.errors.iter().enumerate() {
            f.write_str(if i == 0 { " " } else { "; " })?;
            write!(f, "{error}")?
        }
        Ok(())
    }
}

impl<S: core::fmt::Display> core::fmt::Display for BuildErrorKind<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingComponent { entry, component } => {
                write!(f, "{entry} missing {component}")
            }
            Self::DuplicateComponent { entry, component } => {
                write!(f, "{entry} has duplicate {component}")
            }
            Self::PinOutOfRange { entry, slot, len } => {
                write!(f, "{entry} pinned to {slot} but registry has {len} entries")
            }
            Self::PinConflict {
                first,
                second,
                slot,
            } => {
                write!(f, "slot {slot} pinned to both {first} and {second}")
            }
            Self::DoublePin {
                entry,
                first,
                second,
            } => {
                write!(f, "{entry} pinned to both {first} and {second} slots")
            }
            Self::DuplicateColumn { component } => {
                write!(f, "{component} already exists")
            }
            Self::InsertOutOfRange { component, id, len } => {
                write!(
                    f,
                    "{component} value for id {id} but registry has {len} entries"
                )
            }
            Self::CapExceeded { len, cap } => {
                write!(f, "registry has {len} entries but cap is {cap}")
            }
            Self::DuplicateProperty { entry, property } => {
                write!(f, "{entry} has duplicate property {property}")
            }
            Self::TooManyStates { entry } => {
                write!(f, "{entry} state count overflows u32")
            }
            Self::TooManyStatesTotal { entry } => {
                write!(f, "total state count overflows u32 at {entry}")
            }
        }
    }
}

impl<S: core::fmt::Debug + core::fmt::Display> Error for RegistryError<S> {}
impl<S: core::fmt::Debug + core::fmt::Display> Error for BuildError<S> {}

pub struct RegistryBuilder<K, S = Name> {
    index: BTreeMap<S, u32>,
    next: u32,
    stages: BTreeMap<TypeId, Stage<S>>,
    pins: Vec<(u32, u32)>,
    properties: Vec<(u32, PropertyDeclaration)>,
    cap: Option<u32>,
    _k: PhantomData<fn() -> K>,
}

pub struct RegistryInit<K, S> {
    registry: Registry<K, S>,
    perm: Perm<K>,
}

impl<K, S> Deref for RegistryInit<K, S> {
    type Target = Registry<K, S>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}

impl<K: 'static, S: StableId> RegistryInit<K, S> {
    pub fn perm(&self) -> &Perm<K> {
        &self.perm
    }

    pub fn insert_column<C: Facet>(
        &mut self,
        values: impl IntoIterator<Item = (Id<K>, C)>,
    ) -> Result<(), BuildError<S>> {
        let mut err = BuildError { errors: Vec::new() };
        if self.registry.columns.contains_key(&TypeId::of::<C>()) {
            err.errors.push(BuildErrorKind::DuplicateColumn {
                component: type_name::<C>(),
            });
        }
        let mut pairs = Vec::new();
        let len = self.registry.stable_ids.len() as u32;
        for (id, value) in values {
            if id.raw() >= len {
                err.errors.push(BuildErrorKind::InsertOutOfRange {
                    component: type_name::<C>(),
                    id: id.raw(),
                    len,
                });
            } else {
                pairs.push((id.raw(), value));
            }
        }
        match make_column::<K, C, S>(pairs, &self.registry.stable_ids) {
            Ok(column) if err.errors.is_empty() => {
                self.registry.columns.insert(TypeId::of::<C>(), column);
                Ok(())
            }
            Ok(_) => Err(err),
            Err(mut e) => {
                err.errors.append(&mut e.errors);
                Err(err)
            }
        }
    }

    pub fn build(self) -> Registry<K, S> {
        self.registry
    }
}

struct PropertySlot {
    id: TypeId,
    count: u32,
    stride: u32,
}

struct PropertyDeclaration {
    id: TypeId,
    count: u32,
    name: &'static str,
}

pub struct Perm<K> {
    map: Vec<u32>,
    _k: PhantomData<fn() -> K>,
}

impl<K> Perm<K> {
    pub fn id(&self, provisional: u32) -> Id<K> {
        Id::from_raw(self.map[provisional as usize])
    }

    pub fn permute<T>(&self, items: Vec<T>) -> Vec<T> {
        assert_eq!(items.len(), self.map.len());
        let mut out: Vec<Option<T>> = (0..items.len()).map(|_| None).collect();
        for (i, item) in items.into_iter().enumerate() {
            out[self.map[i] as usize] = Some(item);
        }
        out.into_iter().map(Option::unwrap).collect()
    }
}

pub(crate) type Data = Box<dyn Any + Send + Sync>;
type Finish<S> = fn(Data, &[u32], &[S]) -> Result<Data, BuildError<S>>;

struct Stage<S> {
    data: Data,
    finish: Finish<S>,
}

pub struct Entry<'a, K, S> {
    builder: &'a mut RegistryBuilder<K, S>,
    stable_id: S,
    index: Option<u32>,
}

pub struct EntryRef<'a, K, S> {
    builder: &'a mut RegistryBuilder<K, S>,
    index: u32,
}

impl<'a, K: 'static, S: StableId> Entry<'a, K, S> {
    pub fn create(self) -> Result<EntryRef<'a, K, S>, RegistryError<S>> {
        match self.index {
            Some(_) => Err(RegistryError::<S>::DuplicateStableId(self.stable_id)),
            None => Ok(self.insert()),
        }
    }

    pub fn or_create(self) -> EntryRef<'a, K, S> {
        self.or_init(|e| e)
    }

    pub fn or_init(
        self,
        f: impl FnOnce(EntryRef<'a, K, S>) -> EntryRef<'a, K, S>,
    ) -> EntryRef<'a, K, S> {
        match self.index {
            Some(index) => EntryRef {
                builder: self.builder,
                index,
            },
            None => f(self.insert()),
        }
    }

    pub fn or_with<C: Facet>(self, value: C) -> EntryRef<'a, K, S> {
        self.or_init(|e| e.with(value))
    }

    pub fn get(self) -> Option<EntryRef<'a, K, S>> {
        self.index.map(|index| EntryRef {
            builder: self.builder,
            index,
        })
    }

    fn insert(self) -> EntryRef<'a, K, S> {
        let Entry {
            builder, stable_id, ..
        } = self;
        let index = builder.next;
        builder.next += 1;
        builder.index.insert(stable_id, index);
        EntryRef { builder, index }
    }
}

impl<'a, K: 'static, S: StableId> EntryRef<'a, K, S> {
    pub fn property<P: Property>(self) -> Self {
        const { assert!(P::COUNT > 0, "Property::COUNT must be nonzero") };
        self.builder.properties.push((
            self.index,
            PropertyDeclaration {
                id: TypeId::of::<P>(),
                count: P::COUNT,
                name: type_name::<P>(),
            },
        ));
        self
    }

    pub fn with<C: Facet>(self, value: C) -> Self {
        self.builder.stage::<C>().push((self.index, value));
        self
    }

    pub fn pin(self, id: Id<K>) -> Self {
        self.builder.pins.push((self.index, id.raw()));
        self
    }

    pub fn index(&self) -> u32 {
        self.index
    }
}

pub struct Registry<K, S = Name> {
    stable_ids: Vec<S>,
    columns: BTreeMap<TypeId, Data>,
    sorted: Vec<u32>,
    bases: Vec<u32>,
    slots: Vec<Vec<PropertySlot>>,
    _k: PhantomData<fn() -> K>,
}

impl<K, S> Default for RegistryBuilder<K, S> {
    fn default() -> Self {
        Self {
            index: Default::default(),
            stages: Default::default(),
            next: Default::default(),
            pins: Default::default(),
            properties: Default::default(),
            cap: Default::default(),
            _k: Default::default(),
        }
    }
}

fn make_column<K: 'static, C: Facet, S: StableId>(
    mut pairs: Vec<(u32, C)>,
    stable_ids: &[S],
) -> Result<Data, BuildError<S>> {
    pairs.sort_by_key(|pair| pair.0);
    let mut errors: Vec<BuildErrorKind<S>> = Vec::new();

    for pair in pairs.windows(2) {
        if pair[0].0 == pair[1].0 {
            errors.push(BuildErrorKind::<S>::DuplicateComponent {
                entry: stable_ids[pair[0].0 as usize].clone(),
                component: type_name::<C>(),
            });
        }
    }

    let column = match C::STORAGE {
        Storage::Sparse => {
            let mut ids = Vec::with_capacity(pairs.len());
            let mut values = Vec::with_capacity(pairs.len());
            for (id, value) in pairs {
                ids.push(id);
                values.push(value);
            }
            Column::<K, C>::sparse(ids, values)
        }
        Storage::Dense(fill) => {
            let mut values: Vec<C> = (0..stable_ids.len()).map(|_| fill()).collect();
            for (id, value) in pairs {
                values[id as usize] = value;
            }
            Column::<K, C>::dense(values)
        }
        Storage::Required => {
            let mut idx = 0;
            for (id, stable_id) in stable_ids.iter().enumerate() {
                let present = idx < pairs.len() && pairs[idx].0 == id as u32;
                while idx < pairs.len() && pairs[idx].0 == id as u32 {
                    idx += 1;
                }
                if !present {
                    errors.push(BuildErrorKind::<S>::MissingComponent {
                        entry: stable_id.clone(),
                        component: type_name::<C>(),
                    });
                }
            }
            Column::<K, C>::dense(pairs.into_iter().map(|(_, value)| value).collect())
        }
    };
    if !errors.is_empty() {
        Err(BuildError::<S> { errors })
    } else {
        Ok(Box::new(column))
    }
}

fn finish<K: 'static, C: Facet, S: StableId>(
    data: Data,
    perm: &[u32],
    stable_ids: &[S],
) -> Result<Data, BuildError<S>> {
    let staged: Vec<(u32, C)> = *data.downcast().unwrap();
    let mut pairs: Vec<(u32, C)> = staged
        .into_iter()
        .map(|(old, value)| (perm[old as usize], value))
        .collect();
    pairs.sort_by_key(|pair| pair.0);
    make_column::<K, C, S>(pairs, stable_ids)
}

impl<K: 'static, S: StableId> RegistryBuilder<K, S> {
    pub fn iter(&self) -> impl Iterator<Item = (u32, &S)> {
        self.index
            .iter()
            .map(|(stable_id, &index)| (index, stable_id))
    }

    pub fn declare<C: Facet>(&mut self) {
        self.stage::<C>();
    }

    fn stage<C: Facet>(&mut self) -> &mut Vec<(u32, C)> {
        self.stages
            .entry(TypeId::of::<C>())
            .or_insert_with(|| Stage {
                data: Box::new(Vec::<(u32, C)>::new()),
                finish: finish::<K, C, S>,
            })
            .data
            .downcast_mut()
            .unwrap()
    }

    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn with_cap(mut self, cap: u32) -> Self {
        self.cap = Some(cap);
        self
    }

    pub fn create(&mut self, stable_id: S) -> Result<EntryRef<'_, K, S>, RegistryError<S>> {
        self.entry(stable_id).create()
    }

    pub fn contains(&self, stable_id: &S) -> bool {
        self.index.contains_key(stable_id)
    }

    pub fn entry(&mut self, stable_id: S) -> Entry<'_, K, S> {
        Entry {
            index: self.index.get(&stable_id).copied(),
            builder: self,
            stable_id,
        }
    }

    pub fn build_init(self) -> Result<RegistryInit<K, S>, BuildError<S>> {
        let mut perm = alloc::vec![0u32; self.next as usize];
        let mut by_entry = BTreeMap::new();
        let mut by_slot = BTreeMap::new();
        let mut columns = BTreeMap::new();
        let mut err = BuildError { errors: Vec::new() };
        let mut by_old = alloc::vec![None; self.next as usize];
        let mut stable_id_by_id = by_old.clone();
        for (stable_id, &old) in &self.index {
            by_old[old as usize] = Some(stable_id);
        }
        let mut sorted = Vec::with_capacity(self.next as usize);
        let stable_id_of = |i: u32| by_old[i as usize].unwrap().clone();
        if let Some(cap) = self.cap
            && cap < self.next
        {
            err.errors.push(BuildErrorKind::CapExceeded {
                len: self.next,
                cap,
            });
        }
        for (i, slot) in self.pins {
            if let Some(&first) = by_entry.get(&i) {
                err.errors.push(BuildErrorKind::DoublePin {
                    entry: stable_id_of(i),
                    first,
                    second: slot,
                });
            } else if slot >= self.next {
                err.errors.push(BuildErrorKind::PinOutOfRange {
                    entry: stable_id_of(i),
                    slot,
                    len: self.next,
                });
            } else if let Some(&first) = by_slot.get(&slot) {
                err.errors.push(BuildErrorKind::PinConflict {
                    first: stable_id_of(first),
                    second: stable_id_of(i),
                    slot,
                });
            } else {
                by_entry.insert(i, slot);
                by_slot.insert(slot, i);
            }
        }
        let mut free = (0..self.next).filter(|s| !by_slot.contains_key(s));
        for (stable_id, &old) in &self.index {
            let id = match by_entry.get(&old) {
                Some(&slot) => slot,
                None => free.next().unwrap(),
            };
            perm[old as usize] = id;
            stable_id_by_id[id as usize] = Some(stable_id);
            sorted.push(id);
        }
        let stable_ids: Vec<S> = stable_id_by_id
            .into_iter()
            .map(Option::unwrap)
            .cloned()
            .collect();
        for (type_id, stage) in self.stages {
            match (stage.finish)(stage.data, &perm, &stable_ids) {
                Ok(column) => {
                    columns.insert(type_id, column);
                }
                Err(mut e) => {
                    err.errors.append(&mut e.errors);
                }
            }
        }
        let n = self.next as usize;
        let mut decls: Vec<Vec<PropertyDeclaration>> = (0..n).map(|_| Vec::new()).collect();
        for (i, decl) in self.properties {
            let list = &mut decls[i as usize];
            if list.iter().any(|d| d.id == decl.id) {
                err.errors.push(BuildErrorKind::DuplicateProperty {
                    entry: stable_id_of(i),
                    property: decl.name,
                });
            } else {
                list.push(decl);
            }
        }
        let mut slots: Vec<Vec<PropertySlot>> = (0..n).map(|_| Vec::new()).collect();
        let mut counts = alloc::vec![1u32; n];
        for old in 0..n {
            let mut stride: u32 = 1;
            let mut sl = Vec::with_capacity(decls[old].len());
            let mut overflow = false;
            for d in decls[old].iter().rev() {
                sl.push(PropertySlot {
                    id: d.id,
                    count: d.count,
                    stride,
                });
                match stride.checked_mul(d.count) {
                    Some(s) => stride = s,
                    None => {
                        overflow = true;
                        break;
                    }
                }
            }
            if overflow {
                err.errors.push(BuildErrorKind::TooManyStates {
                    entry: stable_id_of(old as u32),
                });
                continue;
            }
            sl.reverse();
            let id = perm[old] as usize;
            slots[id] = sl;
            counts[id] = stride;
        }
        let mut bases = Vec::with_capacity(n + 1);
        let mut acc: u32 = 0;
        bases.push(0);
        for id in 0..n {
            match acc.checked_add(counts[id]) {
                Some(a) => acc = a,
                None => err.errors.push(BuildErrorKind::TooManyStatesTotal {
                    entry: stable_ids[id].clone(),
                }),
            }
            bases.push(acc);
        }
        if err.errors.is_empty() {
            Ok(RegistryInit {
                registry: Registry {
                    stable_ids,
                    columns,
                    sorted,
                    bases,
                    slots,
                    _k: PhantomData,
                },
                perm: Perm {
                    map: perm,
                    _k: PhantomData,
                },
            })
        } else {
            Err(err)
        }
    }

    pub fn build(self) -> Result<Registry<K, S>, BuildError<S>> {
        Ok(self.build_init()?.build())
    }
}

impl<K: 'static, S: StableId> Registry<K, S> {
    pub fn total_states(&self) -> u32 {
        *self.bases.last().unwrap()
    }

    pub fn default_state(&self, id: Id<K>) -> StateId<K> {
        StateId::from_raw(self.bases[id.index()])
    }

    pub fn state_count(&self, id: Id<K>) -> u32 {
        self.bases[id.index() + 1] - self.bases[id.index()]
    }

    pub fn entry_of(&self, state_id: StateId<K>) -> Id<K> {
        debug_assert!(state_id.raw() < self.total_states());
        Id::from_raw((self.bases.partition_point(|&b| b <= state_id.raw()) - 1) as u32)
    }

    fn slot<P: Property>(&self, id: usize) -> Option<&PropertySlot> {
        self.slots[id].iter().find(|sl| sl.id == TypeId::of::<P>())
    }

    pub fn property<P: Property>(&self, state_id: StateId<K>) -> Option<P> {
        debug_assert!(state_id.raw() < self.total_states());
        let entry = self.entry_of(state_id);
        let slot = self.slot::<P>(entry.index())?;
        let local = state_id.raw() - self.bases[entry.index()];
        let old = (local / slot.stride) % slot.count;
        Some(P::from_raw(old))
    }

    pub fn set<P: Property>(&self, state_id: StateId<K>, value: P) -> Option<StateId<K>> {
        debug_assert!(state_id.raw() < self.total_states());
        let entry = self.entry_of(state_id);
        let slot = self.slot::<P>(entry.index())?;
        let local = state_id.raw() - self.bases[entry.index()];
        let old = (local / slot.stride) % slot.count;
        let new = value.to_raw();
        debug_assert!(new < slot.count);
        Some(StateId::from_raw(
            state_id.raw() - old * slot.stride + new * slot.stride,
        ))
    }

    pub fn builder() -> RegistryBuilder<K, S> {
        RegistryBuilder::new()
    }
    pub fn column<C: Facet>(&self) -> Option<&Column<K, C>> {
        self.columns.get(&TypeId::of::<C>())?.downcast_ref()
    }
    pub fn len(&self) -> usize {
        self.stable_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stable_ids.is_empty()
    }

    pub fn id<Query>(&self, stable_id: &Query) -> Option<Id<K>>
    where
        S: Borrow<Query>,
        Query: Ord + ?Sized,
    {
        self.sorted
            .binary_search_by(|&i| self.stable_ids[i as usize].borrow().cmp(stable_id))
            .ok()
            .map(|i| Id::from_raw(self.sorted[i]))
    }

    pub fn stable_id(&self, id: Id<K>) -> Option<&S> {
        self.stable_ids.get(id.index())
    }

    pub fn stable_ids(&self) -> &[S] {
        &self.stable_ids
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<K>, &S)> {
        self.stable_ids
            .iter()
            .enumerate()
            .map(|(i, n)| (Id::from_raw(i as u32), n))
    }

    pub fn remap<'a>(&self, palette: impl IntoIterator<Item = &'a S>) -> Remap<K, S>
    where
        S: 'a,
    {
        let mut slots = Vec::new();
        let mut missing = Vec::new();
        for (i, stable_id) in palette.into_iter().enumerate() {
            let id = self.id(stable_id);
            if id.is_none() {
                missing.push((i, stable_id.clone()));
            }
            slots.push(id);
        }
        Remap { slots, missing }
    }
}

pub struct Remap<K, S> {
    pub slots: Vec<Option<Id<K>>>,
    pub missing: Vec<(usize, S)>,
}

impl<K, S> Remap<K, S> {
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use core::any::type_name;

    use alloc::vec::Vec;

    use crate::{
        Id, Property, StateId,
        facet::{Facet, Storage},
        name::Name,
        registry::{BuildErrorKind, RegistryBuilder, RegistryError},
    };

    #[derive(Debug)]
    struct Block;

    #[derive(Debug, PartialEq)]
    struct Hardness(u32);
    impl Facet for Hardness {}

    #[derive(Debug, PartialEq)]
    struct Model(u32);
    impl Facet for Model {
        const STORAGE: Storage<Self> = Storage::Required;
    }

    #[derive(Debug, PartialEq)]
    struct Solid(bool);
    impl Facet for Solid {
        const STORAGE: Storage<Self> = Storage::Dense(|| Solid(false));
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    enum Facing {
        North,
        East,
        South,
        West,
    }
    impl Property for Facing {
        const COUNT: u32 = 4;
        fn to_raw(self) -> u32 {
            self as u32
        }
        fn from_raw(raw: u32) -> Self {
            match raw {
                0 => Self::North,
                1 => Self::East,
                2 => Self::South,
                _ => Self::West,
            }
        }
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Lit(bool);
    impl Property for Lit {
        const COUNT: u32 = 2;
        fn to_raw(self) -> u32 {
            u32::from(self.0)
        }
        fn from_raw(raw: u32) -> Self {
            Self(raw == 1)
        }
    }

    struct Huge(u32); // for overflow tests
    impl Property for Huge {
        const COUNT: u32 = u32::MAX;
        fn to_raw(self) -> u32 {
            self.0
        }
        fn from_raw(raw: u32) -> Self {
            Self(raw)
        }
    }

    fn n(s: &str) -> Name {
        s.parse().unwrap()
    }

    #[test]
    fn ids_follow_name_order() {
        let mut b = RegistryBuilder::<Block, Name>::new();
        b.create(n("t:c")).unwrap();
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.id("t:a").unwrap().raw(), 0);
        assert_eq!(r.id("t:b").unwrap().raw(), 1);
        assert_eq!(r.id("t:c").unwrap().raw(), 2);
    }

    #[test]
    fn duplicate_name() {
        let mut b = RegistryBuilder::<Block, Name>::new();
        b.create(n("t:a")).unwrap();
        assert_eq!(
            b.create(n("t:a")).err(),
            Some(RegistryError::DuplicateStableId(n("t:a")))
        )
    }

    #[test]
    fn value_follows_entry_through_the_sort() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().with(Hardness(5));
        b.create(n("t:a")).unwrap();
        let r = b.build().unwrap();
        let col = r.column::<Hardness>().unwrap();
        let c = r.id("t:c").unwrap();
        let a = r.id("t:a").unwrap();
        assert_eq!(col.get(c), Some(&Hardness(5)));
        assert_eq!(col.get(a), None);
    }

    #[test]
    fn dense_fills_missing_entries() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let col = r.column::<Solid>().unwrap();
        assert_eq!(col[r.id("t:a").unwrap()], Solid(true));
        assert_eq!(col[r.id("t:b").unwrap()], Solid(false));
    }

    #[test]
    fn remap_reports_missing() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let remap = r.remap([n("t:b"), n("t:x"), n("t:a")].iter());
        assert_eq!(remap.slots, [r.id("t:b"), None, r.id("t:a")]);
        assert_eq!(remap.missing, [(1, n("t:x"))]);
    }

    #[test]
    fn column_on_never_added() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let hardness_column = r.column::<Hardness>();
        assert!(hardness_column.is_none())
    }

    #[test]
    fn get_on_never_added() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let solid_column = r.column::<Solid>().unwrap();
        let something_solid = solid_column.get(Id::from_raw(100));
        assert!(something_solid.is_none())
    }

    #[test]
    fn get_id_on_unknown_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let c = r.id("t:c");
        assert!(c.is_none())
    }

    #[test]
    fn get_unknown_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        assert!(b.entry(n("t:c")).get().is_none());
        assert!(b.build().unwrap().column::<Hardness>().is_none());
    }

    #[test]
    fn get_known_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        b.entry(n("t:a")).get().unwrap().with(Hardness(15));
        let r = b.build().unwrap();
        let a = r.id("t:a").unwrap();
        assert_eq!(r.column::<Hardness>().unwrap().get(a).unwrap().0, 15);
    }

    #[test]
    fn missing_required() {
        let mut b = RegistryBuilder::<Block>::new();
        b.declare::<Model>();
        b.create(n("t:a")).unwrap().with(Solid(true));
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![
                BuildErrorKind::MissingComponent {
                    entry: n("t:a"),
                    component: type_name::<Model>()
                },
                BuildErrorKind::MissingComponent {
                    entry: n("t:b"),
                    component: type_name::<Model>()
                }
            ]
        )
    }

    #[test]
    fn duplicate_facet() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a"))
            .unwrap()
            .with(Solid(true))
            .with(Solid(false));
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::DuplicateComponent {
                entry: n("t:a"),
                component: type_name::<Solid>()
            }]
        );
    }

    #[test]
    fn require_present_is_not_reported() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap().with(Model(1));
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::MissingComponent {
                entry: n("t:a"),
                component: type_name::<Model>()
            }]
        )
    }

    #[test]
    fn duplicate_does_not_fake_missing() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().with(Model(1)).with(Model(2));
        b.create(n("t:b")).unwrap().with(Model(3));
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::DuplicateComponent {
                entry: n("t:a"),
                component: type_name::<Model>()
            }]
        )
    }

    #[test]
    fn triple_attach_reports_two_duplicates() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a"))
            .unwrap()
            .with(Model(1))
            .with(Model(2))
            .with(Model(3));
        assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2)
    }

    #[test]
    fn pin() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().pin(Id::from_raw(0));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.id("t:c").unwrap().raw(), 0);
        assert_eq!(r.id("t:a").unwrap().raw(), 1);
        assert_eq!(r.id("t:b").unwrap().raw(), 2);
    }

    #[test]
    fn pinned_in_order() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().pin(Id::from_raw(2));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.id("t:a").unwrap().raw(), 0);
        assert_eq!(r.id("t:b").unwrap().raw(), 1);
        assert_eq!(r.id("t:c").unwrap().raw(), 2);
    }

    #[test]
    fn pinned_in_middle() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().pin(Id::from_raw(1));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.id("t:a").unwrap().raw(), 0);
        assert_eq!(r.id("t:c").unwrap().raw(), 1);
        assert_eq!(r.id("t:b").unwrap().raw(), 2);
    }

    #[test]
    fn pinned_out_of_range() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().pin(Id::from_raw(3));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::PinOutOfRange {
                entry: n("t:c"),
                slot: 3,
                len: 3
            }]
        );
    }

    #[test]
    fn pin_conflict() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().pin(Id::from_raw(0));
        b.create(n("t:a")).unwrap().pin(Id::from_raw(0));
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::PinConflict {
                first: n("t:c"),
                second: n("t:a"),
                slot: 0,
            }]
        );
    }

    #[test]
    fn triple_pinned() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c"))
            .unwrap()
            .pin(Id::from_raw(1))
            .pin(Id::from_raw(2))
            .pin(Id::from_raw(9));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![
                BuildErrorKind::DoublePin {
                    entry: n("t:c"),
                    first: 1,
                    second: 2
                },
                BuildErrorKind::DoublePin {
                    entry: n("t:c"),
                    first: 1,
                    second: 9
                }
            ]
        );
    }

    #[test]
    fn double_pinned() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c"))
            .unwrap()
            .pin(Id::from_raw(1))
            .pin(Id::from_raw(9));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::DoublePin {
                entry: n("t:c"),
                first: 1,
                second: 9
            }]
        );
    }

    #[test]
    fn double_pinned_with_same_id() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c"))
            .unwrap()
            .pin(Id::from_raw(1))
            .pin(Id::from_raw(1));
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::DoublePin {
                entry: n("t:c"),
                first: 1,
                second: 1
            }]
        );
    }

    #[test]
    fn double_pinned_twice() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c"))
            .unwrap()
            .pin(Id::from_raw(1))
            .pin(Id::from_raw(9));
        b.create(n("t:a"))
            .unwrap()
            .pin(Id::from_raw(2))
            .pin(Id::from_raw(8));
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![
                BuildErrorKind::DoublePin {
                    entry: n("t:c"),
                    first: 1,
                    second: 9
                },
                BuildErrorKind::DoublePin {
                    entry: n("t:a"),
                    first: 2,
                    second: 8
                }
            ]
        );
    }

    #[test]
    fn facet_follows_pin() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c"))
            .unwrap()
            .pin(Id::from_raw(0))
            .with(Hardness(5));
        b.create(n("t:a")).unwrap();
        let r = b.build().unwrap();
        let c = r.id("t:c").unwrap();
        let a = r.id("t:a").unwrap();
        assert_eq!(r.column::<Hardness>().unwrap().get(c).unwrap().0, 5);
        assert_eq!(r.column::<Hardness>().unwrap().get(a), None);
    }

    #[test]
    fn or_init_runs_once() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).or_init(|e| e.with(Hardness(1)));
        b.entry(n("t:a")).or_init(|e| e.with(Hardness(2)));
        let r = b.build().unwrap();
        let a = r.id("t:a").unwrap();
        assert_eq!(r.column::<Hardness>().unwrap().get(a).unwrap().0, 1);
    }

    #[test]
    fn insert_column_unsorted_input() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        b.create(n("t:c")).unwrap();
        let mut init = b.build_init().unwrap();
        let (a, b_, c) = (
            init.id("t:a").unwrap(),
            init.id("t:b").unwrap(),
            init.id("t:c").unwrap(),
        );
        init.insert_column([(c, Hardness(3)), (a, Hardness(3)), (b_, Hardness(3))])
            .unwrap();
        assert_eq!(
            init.build()
                .column::<Hardness>()
                .unwrap()
                .get(b_)
                .unwrap()
                .0,
            3
        );
    }

    struct Twin(Id<Block>);
    impl Facet for Twin {}

    #[test]
    fn twin() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        let base: Vec<(u32, Name)> = b.iter().map(|(i, nm)| (i, nm.clone())).collect();
        let mut twin_of = alloc::vec![];
        for (i, nm) in base {
            let t = b
                .create(alloc::format!("{}_twin", nm.as_str()).parse().unwrap())
                .unwrap();
            twin_of.push((i, t.index()));
        }
        let mut init = b.build_init().unwrap();
        let pairs: Vec<(Id<Block>, Twin)> = twin_of
            .iter()
            .map(|&(base, twin)| (init.perm().id(base), Twin(init.perm().id(twin))))
            .collect();
        init.insert_column(pairs).unwrap();
        let r = init.build();
        assert_eq!(
            r.column::<Twin>().unwrap()[r.id("t:a").unwrap()].0,
            r.id("t:a_twin").unwrap()
        )
    }

    #[test]
    fn cap() {
        let mut b = RegistryBuilder::<Block>::new().with_cap(2);
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        b.build().unwrap();
    }

    #[test]
    fn cap_too_much() {
        let mut b = RegistryBuilder::<Block>::new().with_cap(1);
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::CapExceeded { len: 2, cap: 1 }]
        )
    }

    #[test]
    fn no_props_states_are_ids() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap();
        b.create(n("t:b")).unwrap();
        b.create(n("t:c")).unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.total_states(), 3);
        for (id, _) in r.iter() {
            assert_eq!(r.default_state(id).raw(), id.raw());
            assert_eq!(r.state_count(id), 1);
            assert_eq!(r.entry_of(r.default_state(id)), id);
        }
    }

    #[test]
    fn air_keeps_state_zero() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:air")).unwrap().pin(Id::from_raw(0));
        b.create(n("t:aa")).unwrap().property::<Facing>();
        let r = b.build().unwrap();
        let air = r.id("t:air").unwrap();
        assert_eq!(air.raw(), 0);
        assert_eq!(r.default_state(air).raw(), 0);
        assert_eq!(r.default_state(r.id("t:aa").unwrap()).raw(), 1);
        assert_eq!(r.total_states(), 5);
    }

    #[test]
    fn mixed_radix_roundtrip() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:furnace"))
            .unwrap()
            .property::<Facing>()
            .property::<Lit>();
        let r = b.build().unwrap();
        let id = r.id("t:furnace").unwrap();
        assert_eq!(r.state_count(id), 8);
        for f in [Facing::North, Facing::East, Facing::South, Facing::West] {
            for lit in [false, true] {
                let s = r
                    .set(r.set(r.default_state(id), f).unwrap(), Lit(lit))
                    .unwrap();
                assert_eq!(r.property::<Facing>(s), Some(f));
                assert_eq!(r.property::<Lit>(s), Some(Lit(lit)));
                assert_eq!(r.entry_of(s), id);
            }
        }
    }

    #[test]
    fn stride_is_declaration_order() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a"))
            .unwrap()
            .property::<Facing>()
            .property::<Lit>();
        let r = b.build().unwrap();
        let id = r.id("t:a").unwrap();
        let s = r.set(r.default_state(id), Facing::East).unwrap();
        let s = r.set(s, Lit(true)).unwrap();
        assert_eq!(s.raw(), 3);
    }

    #[test]
    fn entry_of_boundaries() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().property::<Facing>();
        b.create(n("t:b")).unwrap().property::<Lit>();
        let r = b.build().unwrap();
        let a = r.id("t:a").unwrap();
        let b_ = r.id("t:b").unwrap();
        assert_eq!(r.entry_of(StateId::from_raw(0)), a);
        assert_eq!(r.entry_of(StateId::from_raw(3)), a);
        assert_eq!(r.entry_of(StateId::from_raw(4)), b_);
        assert_eq!(r.entry_of(StateId::from_raw(5)), b_);
    }

    #[test]
    fn property_gating() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().property::<Facing>();
        let r = b.build().unwrap();
        let s = r.default_state(r.id("t:a").unwrap());
        assert_eq!(r.property::<Lit>(s), None);
        assert!(r.set(s, Lit(true)).is_none());
        assert_eq!(r.property::<Facing>(s), Some(Facing::North));
    }

    #[test]
    fn pinned_propful_entry() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:b")).unwrap().property::<Lit>();
        b.create(n("t:a")).unwrap();
        b.create(n("t:z"))
            .unwrap()
            .pin(Id::from_raw(0))
            .property::<Facing>();
        let r = b.build().unwrap();
        assert_eq!(r.default_state(r.id("t:z").unwrap()).raw(), 0);
        assert_eq!(r.default_state(r.id("t:a").unwrap()).raw(), 4);
        assert_eq!(r.default_state(r.id("t:b").unwrap()).raw(), 5);
        assert_eq!(r.total_states(), 7);
    }

    #[test]
    fn minting_order_does_not_matter() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:c")).unwrap().property::<Lit>();
        b.create(n("t:a")).unwrap().property::<Facing>();
        let r = b.build().unwrap();
        assert_eq!(r.state_count(r.id("t:a").unwrap()), 4);
        assert_eq!(r.default_state(r.id("t:c").unwrap()).raw(), 4);
        let s = r
            .set(r.default_state(r.id("t:c").unwrap()), Lit(true))
            .unwrap();
        assert_eq!(s.raw(), 5);
    }

    #[test]
    fn duplicate_property() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a"))
            .unwrap()
            .property::<Facing>()
            .property::<Facing>();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::DuplicateProperty {
                entry: n("t:a"),
                property: type_name::<Facing>()
            }]
        );
    }

    #[test]
    fn state_count_overflow() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a"))
            .unwrap()
            .property::<Huge>()
            .property::<Lit>();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::TooManyStates { entry: n("t:a") }]
        );
    }

    #[test]
    fn total_states_overflow() {
        let mut b = RegistryBuilder::<Block>::new();
        b.create(n("t:a")).unwrap().property::<Huge>();
        b.create(n("t:b")).unwrap().property::<Huge>();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![BuildErrorKind::TooManyStatesTotal { entry: n("t:b") }]
        );
    }

    #[test]
    fn property_errors_accumulate() {
        let mut b = RegistryBuilder::<Block>::new();
        b.declare::<Model>();
        b.create(n("t:a"))
            .unwrap()
            .property::<Facing>()
            .property::<Facing>();
        assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2);
    }
}
