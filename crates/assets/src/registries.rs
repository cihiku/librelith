use core::{
    any::{TypeId, type_name},
    error::Error,
    fmt::Display,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::{BuildError, Name, Registry, RegistryBuilder, RegistryInit, StableId, registry::Data};

#[derive(Default)]
pub struct RegistriesBuilder<S = Name> {
    kinds: BTreeMap<TypeId, (Data, BuildKind<S>, FinishKind, S)>,
    kinds_stable_id: BTreeMap<S, TypeId>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RegistriesError<S> {
    DuplicateKind(&'static str),
    DuplicateKindStableId(S),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegistriesBuildError<S> {
    errors: Vec<RegistriesBuildErrorKind<S>>,
}

impl<S> RegistriesBuildError<S> {
    pub fn errors(&self) -> &[RegistriesBuildErrorKind<S>] {
        &self.errors
    }
}

impl<S: core::fmt::Display + core::fmt::Debug> Error for RegistriesBuildError<S> {}

impl<S: core::fmt::Display> Display for RegistriesBuildError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to build this many kinds {}", self.errors.len())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegistriesBuildErrorKind<S> {
    pub stable_id: S,
    pub build: BuildError<S>,
}

impl<S: core::fmt::Display + core::fmt::Debug + 'static> Error for RegistriesBuildErrorKind<S> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.build)
    }
}

impl<S: core::fmt::Display> Display for RegistriesError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RegistriesError::DuplicateKind(name) => write!(f, "{name} kind already exists"),
            RegistriesError::DuplicateKindStableId(stable_id) => {
                write!(
                    f,
                    "{stable_id} kind stable id already exists under different kind"
                )
            }
        }
    }
}

impl<S: core::fmt::Display> Display for RegistriesBuildErrorKind<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to build kind {}", self.stable_id)
    }
}

impl<S: core::fmt::Display + core::fmt::Debug> Error for RegistriesError<S> {}

type BuildKind<S> = fn(Data) -> Result<Data, BuildError<S>>;
type FinishKind = fn(Data) -> Data;

fn finish_kind<K: 'static, S: StableId>(data: Data) -> Data {
    Box::from(data.downcast::<RegistryInit<K, S>>().unwrap().build())
}

fn build_kind<K: 'static, S: StableId>(data: Data) -> Result<Data, BuildError<S>> {
    let builder: RegistryBuilder<K, S> = *data.downcast().unwrap();
    Ok(Box::from(builder.build_init()?))
}

pub struct Registries<S = Name> {
    registries: BTreeMap<TypeId, (Data, S)>,
}

impl<S: StableId> Registries<S> {
    pub fn builder() -> RegistriesBuilder<S> {
        RegistriesBuilder::new()
    }

    pub fn get<K: 'static>(&self) -> Option<&Registry<K, S>> {
        self.registries.get(&TypeId::of::<K>())?.0.downcast_ref()
    }

    pub fn kind_stable_id<K: 'static>(&self) -> Option<&S> {
        Some(&self.registries.get(&TypeId::of::<K>())?.1)
    }
}

impl<S: StableId> RegistriesBuilder<S> {
    pub fn new() -> Self {
        Self {
            kinds: Default::default(),
            kinds_stable_id: Default::default(),
        }
    }

    pub fn insert_kind<K: 'static>(
        &mut self,
        stable_id: S,
        builder: RegistryBuilder<K, S>,
    ) -> Result<&mut Self, RegistriesError<S>> {
        if self.kinds.contains_key(&TypeId::of::<K>()) {
            return Err(RegistriesError::DuplicateKind(type_name::<K>()));
        }
        if self.kinds_stable_id.contains_key(&stable_id) {
            return Err(RegistriesError::DuplicateKindStableId(stable_id.clone()));
        }
        self.kinds.insert(
            TypeId::of::<K>(),
            (
                Box::from(builder),
                build_kind::<K, S>,
                finish_kind::<K, S>,
                stable_id.clone(),
            ),
        );
        self.kinds_stable_id
            .insert(stable_id.clone(), TypeId::of::<K>());
        Ok(self)
    }

    pub fn add_kind<K: 'static>(
        &mut self,
        stable_id: S,
        f: impl FnOnce(&mut RegistryBuilder<K, S>),
    ) -> Result<&mut Self, RegistriesError<S>> {
        let mut builder = Registry::builder();
        f(&mut builder);
        self.insert_kind(stable_id, builder)
    }

    pub fn kind_mut<K: 'static>(&mut self) -> Option<&mut RegistryBuilder<K, S>> {
        self.kinds.get_mut(&TypeId::of::<K>())?.0.downcast_mut()
    }

    pub fn build_init(self) -> Result<RegistriesInit<S>, RegistriesBuildError<S>> {
        let mut inits = BTreeMap::new();
        let mut err = RegistriesBuildError { errors: Vec::new() };
        for (type_id, (builder, build_kind, finish_kind, stable_id)) in self.kinds {
            match build_kind(builder).map_err(|e| RegistriesBuildErrorKind {
                stable_id: stable_id.clone(),
                build: e,
            }) {
                Ok(registry) => {
                    inits.insert(type_id, (registry, finish_kind, stable_id));
                }
                Err(e) => {
                    err.errors.push(e);
                }
            };
        }
        if err.errors.is_empty() {
            Ok(RegistriesInit { inits })
        } else {
            Err(err)
        }
    }

    pub fn build(self) -> Result<Registries<S>, RegistriesBuildError<S>> {
        Ok(self.build_init()?.build())
    }
}

pub struct RegistriesInit<S> {
    inits: BTreeMap<TypeId, (Data, FinishKind, S)>,
}

impl<S: StableId> RegistriesInit<S> {
    pub fn get<K: 'static>(&self) -> Option<&Registry<K, S>> {
        Some(
            self.inits
                .get(&TypeId::of::<K>())?
                .0
                .downcast_ref::<RegistryInit<K, S>>()?,
        )
    }

    pub fn kind_mut<K: 'static>(&mut self) -> Option<&mut RegistryInit<K, S>> {
        self.inits.get_mut(&TypeId::of::<K>())?.0.downcast_mut()
    }

    pub fn build(self) -> Registries<S> {
        Registries {
            registries: self
                .inits
                .into_iter()
                .map(|(type_id, (data, finish, stable_id))| (type_id, (finish(data), stable_id)))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::any::type_name;

    use crate::{
        BuildError, Facet, Name, Registries, RegistriesError, Storage,
        registries::RegistriesBuildErrorKind, registry::BuildErrorKind,
    };

    struct Block;
    struct Item;
    struct Fluid;

    struct Hardness(u32);
    impl Facet for Hardness {}

    struct Model();
    impl Facet for Model {
        const STORAGE: Storage<Self> = Storage::Required;
    }

    fn n(s: &str) -> Name {
        s.parse().unwrap()
    }

    #[test]
    fn two_kinds_roundtrip() {
        let mut b = Registries::builder();
        b.add_kind::<Block>(n("t:block"), |k| {
            k.create(n("t:stone")).unwrap().with(Hardness(5));
        })
        .unwrap()
        .add_kind::<Item>(n("t:item"), |k| {
            k.create(n("t:stick")).unwrap();
        })
        .unwrap();
        let r = b.build().unwrap();
        let blocks = r.get::<Block>().unwrap();
        let stone = blocks.id("t:stone").unwrap();
        assert_eq!(blocks.column::<Hardness>().unwrap()[stone].0, 5);
        assert_eq!(r.get::<Item>().unwrap().len(), 1);
        assert_eq!(r.kind_stable_id::<Block>().unwrap().as_str(), "t:block");
        assert!(r.get::<Fluid>().is_none());
        assert_eq!(r.kind_stable_id::<Fluid>(), None);
    }

    #[test]
    fn duplicate_kind() {
        let mut b = Registries::builder();
        b.add_kind::<Block>(n("t:block"), |_| {}).unwrap();
        assert_eq!(
            b.add_kind::<Block>(n("t:other"), |_| {}).err().unwrap(),
            RegistriesError::DuplicateKind(type_name::<Block>())
        )
    }

    #[test]
    fn duplicate_kind_name() {
        let mut b = Registries::builder();
        b.add_kind::<Block>(n("t:block"), |_| {}).unwrap();
        assert_eq!(
            b.add_kind::<Item>(n("t:block"), |_| {}).err().unwrap(),
            RegistriesError::DuplicateKindStableId(n("t:block"))
        )
    }

    #[test]
    fn build_error_names_the_kind() {
        let mut b = Registries::builder();
        b.add_kind::<Block>(n("t:block"), |k| {
            k.declare::<Model>();
            k.create(n("t:stone")).unwrap();
        })
        .unwrap()
        .add_kind::<Item>(n("t:item"), |k| {
            k.create(n("t:stick")).unwrap();
        })
        .unwrap();
        assert_eq!(
            b.build().map(|_| ()).unwrap_err().errors,
            alloc::vec![RegistriesBuildErrorKind {
                stable_id: n("t:block"),
                build: BuildError {
                    errors: alloc::vec![BuildErrorKind::MissingComponent {
                        entry: n("t:stone"),
                        component: type_name::<Model>()
                    }]
                }
            }]
        );
    }

    #[test]
    fn two_failing_kinds_both_reported() {
        let mut b = Registries::builder();
        b.add_kind::<Block>(n("t:block"), |k| {
            k.declare::<Model>();
            k.create(n("t:stone")).unwrap();
        })
        .unwrap()
        .add_kind::<Item>(n("t:item"), |k| {
            k.declare::<Model>();
            k.create(n("t:stick")).unwrap();
        })
        .unwrap();
        assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2)
    }
}
