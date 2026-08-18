pub mod view;
pub use view::{ColumnRegistryView, ColumnView};

use crate::StableKey;

pub trait Facet {
    type Key: StableKey + ?Sized + 'static;
    const KEY: &'static Self::Key;
}
