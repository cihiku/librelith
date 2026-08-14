use core::any::TypeId;
pub trait Property: Sized + 'static {
    const COUNT: u32;
    fn to_raw(self) -> u32;
    fn from_raw(raw: u32) -> Self;
}

pub struct PropertySlot {
    pub(crate) id: TypeId,
    pub(crate) count: u32,
    pub(crate) stride: u32,
}

pub struct PropertyDeclaration {
    pub(crate) id: TypeId,
    pub(crate) count: u32,
    pub(crate) name: &'static str,
}
