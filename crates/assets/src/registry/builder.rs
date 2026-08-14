use core::{
    any::{Any, TypeId, type_name},
    marker::PhantomData,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    BuildError, BuildErrorKind, Column, Facet, Id, Name, Property, Registry, RegistryError,
    RegistryInit, StableId, Storage,
    state::{PropertyDeclaration, PropertySlot},
};

pub struct RegistryBuilder<K, S = Name> {
    index: BTreeMap<S, u32>,
    next: u32,
    stages: BTreeMap<TypeId, Stage<S>>,
    pins: Vec<(u32, u32)>,
    properties: Vec<(u32, PropertyDeclaration)>,
    cap: Option<u32>,
    _k: PhantomData<fn() -> K>,
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

struct Stage<S = Name> {
    data: Data,
    finish: Finish<S>,
}

pub struct Entry<'a, K, S = Name> {
    builder: &'a mut RegistryBuilder<K, S>,
    stable_id: S,
    index: Option<u32>,
}

pub struct EntryRef<'a, K, S = Name> {
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

pub(crate) fn make_column<K: 'static, C: Facet, S: StableId>(
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
