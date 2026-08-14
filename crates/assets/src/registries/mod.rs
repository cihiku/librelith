pub mod builder;
pub mod error;

pub use builder::*;
pub use error::*;

#[cfg(test)]
mod tests;
use core::any::TypeId;

use alloc::collections::btree_map::BTreeMap;

use crate::{
    Name, Registry, StableId,
    registry::{AnyRegistry, Data},
};

pub struct Registries<S = Name> {
    registries: BTreeMap<TypeId, (Data, Cast<S>, S)>,
    kinds_stable_id: BTreeMap<S, TypeId>,
}

impl<S: StableId> Registries<S> {
    pub fn builder() -> RegistriesBuilder<S> {
        RegistriesBuilder::new()
    }

    pub fn get<K: 'static>(&self) -> Option<&Registry<K, S>> {
        self.registries.get(&TypeId::of::<K>())?.0.downcast_ref()
    }

    pub fn kind_stable_id<K: 'static>(&self) -> Option<&S> {
        Some(&self.registries.get(&TypeId::of::<K>())?.2)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&S, &dyn AnyRegistry<S>)> {
        self.kinds_stable_id.iter().map(|(s, t)| {
            let (data, cast, _) = &self.registries[t];
            (s, cast(data))
        })
    }
}
