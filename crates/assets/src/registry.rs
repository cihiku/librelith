use core::{
    any::{Any, TypeId, type_name},
    error::Error,
    marker::PhantomData,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    facet::{Column, Facet, Storage},
    id::Id,
    name::Name,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateName(Name),
    UnknownName(Name),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError {
    pub(crate) errors: Vec<BuildErrorKind>,
}

impl BuildError {
    pub fn errors(&self) -> &[BuildErrorKind] {
        &self.errors
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildErrorKind {
    MissingComponent {
        entry: Name,
        component: &'static str,
    },
    DuplicateComponent {
        entry: Name,
        component: &'static str,
    },
}

impl core::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DuplicateName(name) => write!(f, "{name} already in registry"),
            Self::UnknownName(name) => write!(f, "{name} is unknown to registry"),
        }
    }
}

impl core::fmt::Display for BuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let n = self.errors.len();
        write!(f, "{} build errors: ", if n == 1 { "" } else { "s" })?;
        for (i, error) in self.errors.iter().enumerate() {
            f.write_str(if i == 0 { " " } else { "; " })?;
            write!(f, "{error}")?
        }
        Ok(())
    }
}

impl core::fmt::Display for BuildErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingComponent { entry, component } => {
                write!(f, "{entry} missing {component}")
            }
            Self::DuplicateComponent { entry, component } => {
                write!(f, "{entry} has duplicate {component}")
            }
        }
    }
}

impl Error for RegistryError {}
impl Error for BuildError {}

pub struct RegistryBuilder<K> {
    index: BTreeMap<Name, u32>,
    next: u32,
    stages: BTreeMap<TypeId, Stage>,
    _k: PhantomData<fn() -> K>,
}

pub(crate) type Data = Box<dyn Any + Send + Sync>;
type Finish = fn(Data, &[u32], &[Name]) -> Result<Data, BuildError>;

struct Stage {
    data: Data,
    finish: Finish,
}

pub struct EntryRef<'a, K> {
    builder: &'a mut RegistryBuilder<K>,
    index: u32,
}

impl<'a, K: 'static> EntryRef<'a, K> {
    pub fn with<C: Facet>(self, value: C) -> Self {
        self.builder.stage::<C>().push((self.index, value));
        self
    }
}

pub struct Registry<K> {
    names: Vec<Name>,
    columns: BTreeMap<TypeId, Data>,
    _k: PhantomData<fn() -> K>,
}

impl<K> Default for RegistryBuilder<K> {
    fn default() -> Self {
        Self {
            index: Default::default(),
            stages: Default::default(),
            next: Default::default(),
            _k: PhantomData,
        }
    }
}

fn finish<K: 'static, C: Facet>(
    data: Data,
    perm: &[u32],
    names: &[Name],
) -> Result<Data, BuildError> {
    let staged: Vec<(u32, C)> = *data.downcast().unwrap();
    let mut pairs: Vec<(u32, C)> = staged
        .into_iter()
        .map(|(old, value)| (perm[old as usize], value))
        .collect();
    pairs.sort_by_key(|pair| pair.0);

    let mut errors: Vec<BuildErrorKind> = Vec::new();

    for pair in pairs.windows(2) {
        if pair[0].0 == pair[1].0 {
            errors.push(BuildErrorKind::DuplicateComponent {
                entry: names[pair[0].0 as usize].clone(),
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
            let mut values: Vec<C> = (0..names.len()).map(|_| fill()).collect();
            for (id, value) in pairs {
                values[id as usize] = value;
            }
            Column::<K, C>::dense(values)
        }
        Storage::Required => {
            let mut idx = 0;
            for (id, name) in names.iter().enumerate() {
                let present = idx < pairs.len() && pairs[idx].0 == id as u32;
                while idx < pairs.len() && pairs[idx].0 == id as u32 {
                    idx += 1;
                }
                if !present {
                    errors.push(BuildErrorKind::MissingComponent {
                        entry: name.clone(),
                        component: type_name::<C>(),
                    });
                }
            }
            Column::<K, C>::dense(pairs.into_iter().map(|(_, value)| value).collect())
        }
    };

    if !errors.is_empty() {
        Err(BuildError { errors })
    } else {
        Ok(Box::new(column))
    }
}

impl<K: 'static> RegistryBuilder<K> {
    pub fn attach<C: Facet>(&mut self, name: &Name, value: C) -> Result<(), RegistryError> {
        let Some(index) = self.index.get(name).cloned() else {
            return Err(RegistryError::UnknownName(name.clone()));
        };
        self.stage::<C>().push((index, value));
        Ok(())
    }
    pub fn declare<C: Facet>(&mut self) {
        self.stage::<C>();
    }
    fn stage<C: Facet>(&mut self) -> &mut Vec<(u32, C)> {
        self.stages
            .entry(TypeId::of::<C>())
            .or_insert_with(|| Stage {
                data: Box::new(Vec::<(u32, C)>::new()),
                finish: finish::<K, C>,
            })
            .data
            .downcast_mut()
            .unwrap()
    }

    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
            stages: BTreeMap::new(),
            next: 0,
            _k: PhantomData,
        }
    }

    pub fn entry(&mut self, name: Name) -> Result<EntryRef<'_, K>, RegistryError> {
        if self.index.contains_key(&name) {
            return Err(RegistryError::DuplicateName(name));
        }
        let index = self.next;
        self.next += 1;
        self.index.insert(name, index);
        Ok(EntryRef {
            builder: self,
            index,
        })
    }

    pub fn build(self) -> Result<Registry<K>, BuildError> {
        let mut perm = alloc::vec![0u32; self.next as usize];
        let mut names = Vec::with_capacity(self.next as usize);
        for (id, (name, &old)) in self.index.iter().enumerate() {
            perm[old as usize] = id as u32;
            names.push(name.clone());
        }
        let mut columns = BTreeMap::new();
        let mut err = BuildError { errors: Vec::new() };
        for (type_id, stage) in self.stages {
            match (stage.finish)(stage.data, &perm, &names) {
                Ok(column) => {
                    columns.insert(type_id, column);
                }
                Err(mut e) => {
                    err.errors.append(&mut e.errors);
                }
            }
        }
        if err.errors.is_empty() {
            Ok(Registry {
                names,
                columns,
                _k: PhantomData,
            })
        } else {
            Err(err)
        }
    }
}

impl<K: 'static> Registry<K> {
    pub fn builder() -> RegistryBuilder<K> {
        RegistryBuilder::new()
    }
    pub fn column<C: Facet>(&self) -> Option<&Column<K, C>> {
        self.columns.get(&TypeId::of::<C>())?.downcast_ref()
    }
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn id(&self, name: &Name) -> Option<Id<K>> {
        self.names
            .binary_search(name)
            .ok()
            .map(|i| Id::from_raw(i as u32))
    }

    pub fn name(&self, id: Id<K>) -> Option<&Name> {
        self.names.get(id.index())
    }

    pub fn names(&self) -> &[Name] {
        &self.names
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<K>, &Name)> {
        self.names
            .iter()
            .enumerate()
            .map(|(i, n)| (Id::from_raw(i as u32), n))
    }

    pub fn remap<'a>(&self, palette: impl IntoIterator<Item = &'a Name>) -> Remap<K> {
        let mut slots = Vec::new();
        let mut missing = Vec::new();
        for (i, name) in palette.into_iter().enumerate() {
            let id = self.id(name);
            if id.is_none() {
                missing.push((i, name.clone()));
            }
            slots.push(id);
        }
        Remap { slots, missing }
    }
}

pub struct Remap<K> {
    pub slots: Vec<Option<Id<K>>>,
    pub missing: Vec<(usize, Name)>,
}

impl<K> Remap<K> {
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use core::any::type_name;

    use crate::{
        Id,
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

    fn n(s: &str) -> Name {
        s.parse().unwrap()
    }

    #[test]
    fn ids_follow_name_order() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:c")).unwrap();
        b.entry(n("t:a")).unwrap();
        b.entry(n("t:b")).unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.id(&n("t:a")).unwrap().raw(), 0);
        assert_eq!(r.id(&n("t:b")).unwrap().raw(), 1);
        assert_eq!(r.id(&n("t:c")).unwrap().raw(), 2);
    }

    #[test]
    fn duplicate_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap();
        assert_eq!(
            b.entry(n("t:a")).err(),
            Some(RegistryError::DuplicateName(n("t:a")))
        )
    }

    #[test]
    fn value_follows_entry_through_the_sort() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:c")).unwrap().with(Hardness(5));
        b.entry(n("t:a")).unwrap();
        let r = b.build().unwrap();
        let col = r.column::<Hardness>().unwrap();
        let c = r.id(&n("t:c")).unwrap();
        let a = r.id(&n("t:a")).unwrap();
        assert_eq!(col.get(c), Some(&Hardness(5)));
        assert_eq!(col.get(a), None);
    }

    #[test]
    fn dense_fills_missing_entries() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let col = r.column::<Solid>().unwrap();
        assert_eq!(col[r.id(&n("t:a")).unwrap()], Solid(true));
        assert_eq!(col[r.id(&n("t:b")).unwrap()], Solid(false));
    }

    #[test]
    fn remap_reports_missing() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap();
        b.entry(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let remap = r.remap([n("t:b"), n("t:x"), n("t:a")].iter());
        assert_eq!(remap.slots, [r.id(&n("t:b")), None, r.id(&n("t:a"))]);
        assert_eq!(remap.missing, [(1, n("t:x"))]);
    }

    #[test]
    fn column_on_never_added() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let hardness_column = r.column::<Hardness>();
        assert!(hardness_column.is_none())
    }

    #[test]
    fn get_on_never_added() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let solid_column = r.column::<Solid>().unwrap();
        let something_solid = solid_column.get(Id::from_raw(100));
        assert!(something_solid.is_none())
    }

    #[test]
    fn get_id_on_unknown_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
        let r = b.build().unwrap();
        let c = r.id(&n("t:c"));
        assert!(c.is_none())
    }

    #[test]
    fn attach_unknown_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
        assert_eq!(
            b.attach(&n("t:c"), Hardness(15)),
            Err(RegistryError::UnknownName(n("t:c")))
        );
        b.build().unwrap();
    }

    #[test]
    fn attach_known_name() {
        let mut b = RegistryBuilder::<Block>::new();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
        assert!(b.attach(&n("t:a"), Hardness(15)).is_ok());
        let r = b.build().unwrap();
        let a = r.id(&n("t:a")).unwrap();
        assert_eq!(r.column::<Hardness>().unwrap().get(a).unwrap().0, 15);
    }

    #[test]
    fn missing_required() {
        let mut b = RegistryBuilder::<Block>::new();
        b.declare::<Model>();
        b.entry(n("t:a")).unwrap().with(Solid(true));
        b.entry(n("t:b")).unwrap();
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
        b.entry(n("t:a"))
            .unwrap()
            .with(Solid(true))
            .with(Solid(false));
        b.entry(n("t:b")).unwrap();
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
        b.entry(n("t:a")).unwrap();
        b.entry(n("t:b")).unwrap().with(Model(1));
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
        b.entry(n("t:a")).unwrap().with(Model(1)).with(Model(2));
        b.entry(n("t:b")).unwrap().with(Model(3));
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
        b.entry(n("t:a"))
            .unwrap()
            .with(Model(1))
            .with(Model(2))
            .with(Model(3));
        assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2)
    }
}
