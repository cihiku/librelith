use core::{
    any::{TypeId, type_name},
    ops::Deref,
};

use alloc::vec::Vec;

use crate::{
    BuildError, BuildErrorKind, Facet, Id, Name, Perm, Registry, StableId, registry::make_column,
};

pub struct RegistryInit<K, S = Name> {
    pub(crate) registry: Registry<K, S>,
    pub(crate) perm: Perm<K>,
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
