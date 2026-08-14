pub mod builder;
pub mod error;
pub mod init;

#[cfg(test)]
mod tests;

pub use builder::*;
use core::{any::TypeId, borrow::Borrow, marker::PhantomData};
pub use error::*;
pub use init::*;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    Name, Property, StableId, StateId,
    facet::{Column, Facet},
    id::Id,
    state::PropertySlot,
};

pub struct Registry<K, S = Name> {
    stable_ids: Vec<S>,
    columns: BTreeMap<TypeId, Data>,
    sorted: Vec<u32>,
    bases: Vec<u32>,
    slots: Vec<Vec<PropertySlot>>,
    _k: PhantomData<fn() -> K>,
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

impl<K: 'static, S: StableId + Borrow<str>> Registry<K, S> {
    pub fn range<'r>(
        &'r self,
        prefix: &str,
    ) -> impl Iterator<Item = (Id<K>, &'r S)> + use<'r, K, S> {
        let start = self
            .sorted
            .partition_point(|&i| self.stable_ids[i as usize].borrow() < prefix);
        let len = self.sorted[start..]
            .partition_point(|&i| self.stable_ids[i as usize].borrow().starts_with(prefix));
        self.sorted[start..start + len]
            .iter()
            .map(|&i| (Id::from_raw(i), &self.stable_ids[i as usize]))
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

pub trait AnyRegistry<S> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn stable_ids(&self) -> &[S];
    fn total_states(&self) -> u32;
}

impl<K: 'static, S: StableId> AnyRegistry<S> for Registry<K, S> {
    fn len(&self) -> usize {
        self.len()
    }

    fn stable_ids(&self) -> &[S] {
        self.stable_ids()
    }

    fn total_states(&self) -> u32 {
        self.total_states()
    }
}
