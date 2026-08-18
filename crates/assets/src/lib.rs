#![no_std]
extern crate alloc;

pub mod entry;
pub mod facet;
pub mod id;
pub mod key;
pub mod registry;
pub mod state;

pub use state::Slot;

pub use entry::EntryRegistryView;
pub use facet::{ColumnRegistryView, ColumnView, Facet};
pub use id::{
    AnyEntryId, AnyRawId, AnyStateId, EntryId, EntrySpace, Erased, FacetSpace, IdSpace, KindSpace,
    PropertySpace, RawId, StateId, StateSpace, ValueSpace,
};
pub use key::{KeyTable, StableKey};
pub use registry::{Kind, RegistryView};
pub use state::{Property, StateRegistryView};
