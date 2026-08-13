use core::{
    any::{TypeId, type_name},
    error::Error,
    fmt::Display,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::{BuildError, Name, Registry, RegistryBuilder, RegistryInit, registry::Data};

#[derive(Default)]
pub struct RegistriesBuilder {
    kinds: BTreeMap<TypeId, (Data, BuildKind, FinishKind, Name)>,
    kinds_name: BTreeMap<Name, TypeId>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RegistriesError {
    DuplicateKind(&'static str),
    DuplicateKindName(Name),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegistriesBuildError {
    errors: Vec<RegistriesBuildErrorKind>,
}

impl RegistriesBuildError {
    pub fn errors(&self) -> &[RegistriesBuildErrorKind] {
        &self.errors
    }
}

impl Error for RegistriesBuildError {}

impl Display for RegistriesBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to build this many kinds {}", self.errors.len())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegistriesBuildErrorKind {
    pub name: Name,
    pub build: BuildError,
}

impl Error for RegistriesBuildErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.build)
    }
}

impl Display for RegistriesError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RegistriesError::DuplicateKind(name) => write!(f, "{name} kind already exists"),
            RegistriesError::DuplicateKindName(name) => {
                write!(f, "{name} kind name already exists under different kind")
            }
        }
    }
}

impl Display for RegistriesBuildErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to build kind {}", self.name)
    }
}

impl Error for RegistriesError {}

type BuildKind = fn(Data) -> Result<Data, BuildError>;
type FinishKind = fn(Data) -> Data;

fn finish_kind<K: 'static>(data: Data) -> Data {
    Box::from(data.downcast::<RegistryInit<K>>().unwrap().build())
}

fn build_kind<K: 'static>(data: Data) -> Result<Data, BuildError> {
    let builder: RegistryBuilder<K> = *data.downcast().unwrap();
    Ok(Box::from(builder.build_init()?))
}

pub struct Registries {
    registries: BTreeMap<TypeId, (Data, Name)>,
}

impl Registries {
    pub fn builder() -> RegistriesBuilder {
        RegistriesBuilder::new()
    }

    pub fn get<K: 'static>(&self) -> Option<&Registry<K>> {
        self.registries.get(&TypeId::of::<K>())?.0.downcast_ref()
    }

    pub fn kind_name<K: 'static>(&self) -> Option<&Name> {
        Some(&self.registries.get(&TypeId::of::<K>())?.1)
    }
}

impl RegistriesBuilder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_kind<K: 'static>(
        &mut self,
        name: Name,
        f: impl FnOnce(&mut RegistryBuilder<K>),
    ) -> Result<&mut Self, RegistriesError> {
        if self.kinds.contains_key(&TypeId::of::<K>()) {
            return Err(RegistriesError::DuplicateKind(type_name::<K>()));
        }
        if self.kinds_name.contains_key(&name) {
            return Err(RegistriesError::DuplicateKindName(name));
        }
        let mut builder = Registry::builder();
        f(&mut builder);
        self.kinds.insert(
            TypeId::of::<K>(),
            (
                Box::from(builder),
                build_kind::<K>,
                finish_kind::<K>,
                name.clone(),
            ),
        );
        self.kinds_name.insert(name, TypeId::of::<K>());
        Ok(self)
    }

    pub fn kind_mut<K: 'static>(&mut self) -> Option<&mut RegistryBuilder<K>> {
        self.kinds.get_mut(&TypeId::of::<K>())?.0.downcast_mut()
    }

    pub fn build_init(self) -> Result<RegistriesInit, RegistriesBuildError> {
        let mut inits = BTreeMap::new();
        let mut err = RegistriesBuildError { errors: Vec::new() };
        for (type_id, (builder, build_kind, finish_kind, name)) in self.kinds {
            match build_kind(builder).map_err(|e| RegistriesBuildErrorKind {
                name: name.clone(),
                build: e,
            }) {
                Ok(registry) => {
                    inits.insert(type_id, (registry, finish_kind, name));
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

    pub fn build(self) -> Result<Registries, RegistriesBuildError> {
        Ok(self.build_init()?.build())
    }
}

pub struct RegistriesInit {
    inits: BTreeMap<TypeId, (Data, FinishKind, Name)>,
}

impl RegistriesInit {
    pub fn get<K: 'static>(&self) -> Option<&Registry<K>> {
        Some(
            self.inits
                .get(&TypeId::of::<K>())?
                .0
                .downcast_ref::<RegistryInit<K>>()?,
        )
    }

    pub fn kind_mut<K: 'static>(&mut self) -> Option<&mut RegistryInit<K>> {
        self.inits.get_mut(&TypeId::of::<K>())?.0.downcast_mut()
    }

    pub fn build(self) -> Registries {
        Registries {
            registries: self
                .inits
                .into_iter()
                .map(|(type_id, (data, finish, name))| (type_id, (finish(data), name)))
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
        assert_eq!(r.kind_name::<Block>().unwrap().as_str(), "t:block");
        assert!(r.get::<Fluid>().is_none());
        assert_eq!(r.kind_name::<Fluid>(), None);
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
            RegistriesError::DuplicateKindName(n("t:block"))
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
                name: n("t:block"),
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
