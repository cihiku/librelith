pub mod view;
pub use view::StateRegistryView;

use crate::StableKey;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Slot {
    pub property: u32,
    pub count: u32,
    pub stride: u32,
}

pub trait Property {
    type Key: StableKey + ?Sized + 'static;
    const KEY: &'static Self::Key;
}
