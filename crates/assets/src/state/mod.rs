pub mod view;
pub use view::StateRegistryView;

#[repr(C)]
pub struct Slot {
    pub property: u32,
    pub count: u32,
    pub stride: u32,
}
