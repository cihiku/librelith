use core::{any::type_name, hash::Hash, marker::PhantomData};

/// Valid only for lifetime of `Registry`
pub struct Id<K>(u32, PhantomData<fn() -> K>);

impl<K> Id<K> {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw, PhantomData)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl<K> Clone for Id<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K> Copy for Id<K> {}

impl<K> PartialEq for Id<K> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K> Eq for Id<K> {}

impl<K> PartialOrd for Id<K> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K> Ord for Id<K> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<K> Hash for Id<K> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<K> core::fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let kind = type_name::<K>().rsplit("::").next().unwrap_or("?");
        write!(f, "Id<{kind}>({})", self.0)
    }
}
