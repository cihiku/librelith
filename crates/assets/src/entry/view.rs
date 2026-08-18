use crate::{AnyEntryId, EntrySpace, KeyTable};

#[derive(Clone, Copy)]
pub struct EntryRegistryView<'a> {
    /// `entries[AnyEntryId.index()]` is entry `id`'s key.
    pub entries: KeyTable<'a, EntrySpace>,
    /// `sorted[i]` is the AnyEntryId whose key ranks `i`-th.
    pub sorted: &'a [AnyEntryId],
}
