use crate::{AnyEntryId, Keyspace};

pub struct EntryRegistryView<'a, V: Keyspace> {
    /// `entries[AnyEntryId.index()]` is entry `id`'s key.
    pub entries: &'a [V::Entry],
    /// `sorted[i]` is the AnyEntryId whose key ranks `i`-th.
    pub sorted: &'a [AnyEntryId],
}
