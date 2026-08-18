use crate::{Keyspace, Slot};

pub struct StateRegistryView<'a, V: Keyspace> {
    /// `bases[AnyEntryId.index()]` Entry `e` owns states
    /// `bases[e]..bases[e + 1]`; `bases[n]` is the total.
    pub bases: &'a [u32],
    /// prefix offsets into `slots`: entry `e`'s slots are
    /// `slots[slot_offsets[e]..slot_offsets[e + 1]]`.
    pub slot_offsets: &'a [u32],
    /// Slot runs, `slot_offsets`-indexed.
    pub slots: &'a [Slot],
    /// Distinct property keys, sorted. `Slot::property` is index.
    pub properties: &'a [V::Property],
    /// Value keys for all properties: property `p`'s
    /// values are `values[value_runs[p]..value_runs[p+1]]`,
    /// and value `raw` is `values[value_runs[p] + raw]`.
    /// Run length equals `Slot::count`
    pub values: &'a [V::Value],
    /// prefix offsets into `values`
    pub value_runs: &'a [u32],
}

impl<'a, V: Keyspace> Clone for StateRegistryView<'a, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, V: Keyspace> Copy for StateRegistryView<'a, V> {}
