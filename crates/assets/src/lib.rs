#![no_std]
extern crate alloc;

pub mod id;
pub use id::{
    AnyEntryId, AnyStateId, EntryId, EntrySpace, Erased, IdSpace, RawId, StateId, StateSpace,
};
