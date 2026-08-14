pub trait Property: Sized + 'static {
    const COUNT: u32;
    fn to_raw(self) -> u32;
    fn from_raw(raw: u32) -> Self;
}
