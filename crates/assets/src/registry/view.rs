use crate::{ColumnRegistryView, EntryRegistryView, Keyspace, StateRegistryView};

pub struct RegistryView<'a, V: Keyspace> {
    pub kind: Option<&'a V::Kind>,
    pub entries: EntryRegistryView<'a, V>,
    pub states: Option<StateRegistryView<'a, V>>,
    pub columns: Option<ColumnRegistryView<'a, V>>,
}

impl<'a, V: Keyspace> Clone for RegistryView<'a, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, V: Keyspace> Copy for RegistryView<'a, V> {}
