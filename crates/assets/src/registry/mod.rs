pub mod view;
pub use view::RegistryView;

use crate::StableKey;

pub trait Kind {
    type Key: StableKey + ?Sized + 'static;
    const KEY: &'static Self::Key;
}
