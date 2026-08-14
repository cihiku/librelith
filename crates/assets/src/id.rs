use core::{
    any::type_name,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// Valid only for lifetime of `Registry`
pub struct Id<K>(u32, PhantomData<fn() -> K>);
pub struct StateId<K>(u32, PhantomData<fn() -> K>);

impl<K> StateId<K> {
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

impl<K> Clone for StateId<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K> Copy for StateId<K> {}

impl<K> PartialEq for StateId<K> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K> Eq for StateId<K> {}

impl<K> PartialOrd for StateId<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K> Ord for StateId<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<K> Hash for StateId<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<K> Debug for StateId<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let kind = type_name::<K>().rsplit("::").next().unwrap_or("?");
        write!(f, "StateId<{kind}>({})", self.0)
    }
}

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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K> Ord for Id<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<K> Hash for Id<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<K> Debug for Id<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let kind = type_name::<K>().rsplit("::").next().unwrap_or("?");
        write!(f, "Id<{kind}>({})", self.0)
    }
}
