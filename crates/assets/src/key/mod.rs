use core::hash::Hash;

pub trait StableKey: Ord + Clone + Hash + Send + Sync + 'static {}
impl<T: Ord + Clone + Hash + Send + Sync + 'static> StableKey for T {}

pub trait Keyspace {
    type Kind: StableKey;
    type Entry: StableKey;
    type Property: StableKey;
    type Value: StableKey;
    type Facet: StableKey;
}
