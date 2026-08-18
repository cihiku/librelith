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
pub use facet::{ColumnRegistryView, ColumnView};
pub use id::{
    AnyEntryId, AnyRawId, AnyStateId, EntryId, EntrySpace, Erased, IdSpace, RawId, StateId,
    StateSpace,
};
pub use key::{Keyspace, StableKey};
pub use registry::RegistryView;
pub use state::StateRegistryView;
