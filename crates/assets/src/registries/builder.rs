use core::any::{TypeId, type_name};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    BuildError, Name, Registries, RegistriesBuildError, RegistriesBuildErrorKind, RegistriesError,
    Registry, RegistryBuilder, RegistryInit, StableId,
    registry::{AnyRegistry, Data},
};
type ConstructionData<S> = (Data, BuildKind<S>, FinishKind, Cast<S>, S);

#[derive(Default)]
pub struct RegistriesBuilder<S = Name> {
    kinds: BTreeMap<TypeId, ConstructionData<S>>,
    kinds_stable_id: BTreeMap<S, TypeId>,
}

type BuildKind<S> = fn(Data) -> Result<Data, BuildError<S>>;
type FinishKind = fn(Data) -> Data;
pub(crate) type Cast<S> = fn(&Data) -> &dyn AnyRegistry<S>;

fn cast_kind<K: 'static, S: StableId>(data: &Data) -> &dyn AnyRegistry<S> {
    data.downcast_ref::<Registry<K, S>>().unwrap()
}

fn finish_kind<K: 'static, S: StableId>(data: Data) -> Data {
    Box::from(data.downcast::<RegistryInit<K, S>>().unwrap().build())
}

fn build_kind<K: 'static, S: StableId>(data: Data) -> Result<Data, BuildError<S>> {
    let builder: RegistryBuilder<K, S> = *data.downcast().unwrap();
    Ok(Box::from(builder.build_init()?))
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
                cast_kind::<K, S>,
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
        let mut kinds_stable_id = BTreeMap::new();
        let mut err = RegistriesBuildError { errors: Vec::new() };
        for (type_id, (builder, build_kind, finish_kind, cast_kind, stable_id)) in self.kinds {
            match build_kind(builder).map_err(|e| RegistriesBuildErrorKind {
                stable_id: stable_id.clone(),
                build: e,
            }) {
                Ok(registry) => {
                    inits.insert(
                        type_id,
                        (registry, finish_kind, cast_kind, stable_id.clone()),
                    );
                    kinds_stable_id.insert(stable_id, type_id);
                }
                Err(e) => {
                    err.errors.push(e);
                }
            };
        }
        if err.errors.is_empty() {
            Ok(RegistriesInit {
                inits,
                kinds_stable_id,
            })
        } else {
            Err(err)
        }
    }

    pub fn build(self) -> Result<Registries<S>, RegistriesBuildError<S>> {
        Ok(self.build_init()?.build())
    }
}

pub struct RegistriesInit<S = Name> {
    inits: BTreeMap<TypeId, (Data, FinishKind, Cast<S>, S)>,
    kinds_stable_id: BTreeMap<S, TypeId>,
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
                .map(|(type_id, (data, finish, cast, stable_id))| {
                    (type_id, (finish(data), cast, stable_id))
                })
                .collect(),
            kinds_stable_id: self.kinds_stable_id,
        }
    }
}
