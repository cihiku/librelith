use crate::{ColumnRegistryView, EntryRegistryView, StateRegistryView};

pub struct RegistryView<'a> {
    pub kind: Option<&'a ()>,
    pub entries: EntryRegistryView<'a>,
    pub states: Option<StateRegistryView<'a>>,
    pub columns: Option<ColumnRegistryView<'a>>,
}

impl<'a> Clone for RegistryView<'a> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a> Copy for RegistryView<'a> {}
