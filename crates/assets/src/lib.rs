//! Data model for game content.
//!
//! `Id<K>` (`u32`) and `Name` (`"namespace:path"`) are handles to game content.
//! `Name` is stable across sessions. Its role to be used in files, network and packs.
//! `Id<K>` is hot runtime handle. Valid only for the `Registry<K>` that produced it.
//!
//! `Registry<K>` maps between the two. Ids are assigned deterministically from the sorted name set.
//!
//! Entries carry ECS-style components (see `facet`), stored as per-component columns inside
//! each registry.
//!
//! ```
//! use librelith_assets::{Facet, Name, Registries, Storage};
//!
//! struct Block;
//! struct Item;
//!
//! struct Hardness(u32);
//! impl Facet for Hardness {}
//!
//! struct LightSource(u32);
//! impl Facet for LightSource {}
//! struct Solid(bool);
//! impl Facet for Solid {
//!     const STORAGE: Storage<Self> = Storage::Dense(|| Solid(false));
//! }
//! struct Texture(u32);
//! impl Facet for Texture {
//!     const STORAGE: Storage<Self> = Storage::Required;
//! }
//!
//! fn n(s: &str) -> Name {
//!     s.parse().unwrap()
//! }
//!
//! fn init() -> Registries {
//!     let mut builder = Registries::builder();
//!     builder
//!         .add_kind::<Block>(n("librelith:block"), |k| {
//!             k.declare::<Hardness>();
//!             k.declare::<LightSource>();
//!             k.declare::<Solid>();
//!             k.entry(n("librelith:stone"))
//!                 .unwrap()
//!                 .with(Hardness(10))
//!                 .with(Texture(10));
//!             k.entry(n("librelith:glowstone"))
//!                 .unwrap()
//!                 .with(Hardness(20))
//!                 .with(Texture(10))
//!                 .with(LightSource(15));
//!         })
//!         .unwrap()
//!         .add_kind::<Item>(n("librelith:item"), |k| {
//!             k.entry(n("librelith:stick")).unwrap();
//!         })
//!         .unwrap();
//!     builder.build().unwrap()
//! }
//!
//! fn usage() {
//!     let registries = init();
//!     let block_registry = registries.get::<Block>().unwrap();
//!     let stone_id = block_registry.id(&n("librelith:stone")).unwrap();
//!     let glowstone_id = block_registry.id(&n("librelith:glowstone")).unwrap();
//!     let stone_hardness = block_registry
//!         .column::<Hardness>()
//!         .unwrap()
//!         .get(stone_id)
//!         .unwrap();
//!     let glowstone_hardness = block_registry
//!         .column::<Hardness>()
//!         .unwrap()
//!         .get(glowstone_id)
//!         .unwrap();
//!     assert_eq!(stone_hardness.0, 10);
//!     assert_eq!(glowstone_hardness.0, 20);
//! }
//! usage();
//! ```

#![no_std]
extern crate alloc;

pub mod facet;
pub mod id;
pub mod name;
pub mod registries;
pub mod registry;

pub use facet::Column;
pub use facet::Facet;
pub use facet::Storage;
pub use id::Id;
pub use name::Name;
pub use registries::Registries;
pub use registries::RegistriesBuilder;
pub use registries::RegistriesError;
pub use registry::BuildError;
pub use registry::EntryRef;
pub use registry::Registry;
pub use registry::RegistryBuilder;
pub use registry::RegistryError;
pub use registry::Remap;
