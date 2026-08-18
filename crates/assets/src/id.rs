use core::{
    any::type_name,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

#[repr(transparent)]
pub struct RawId<P, K>(u32, PhantomData<fn() -> (P, K)>);
pub type AnyRawId<P> = RawId<P, Erased>;

pub enum EntrySpace {}
pub enum StateSpace {}
pub enum PropertySpace {}
pub enum ValueSpace {}
pub enum FacetSpace {}
pub enum KindSpace {}

pub enum Erased {}

pub trait IdSpace {
    const LABEL: &'static str;
}
impl IdSpace for EntrySpace {
    const LABEL: &'static str = "EntryId";
}

impl IdSpace for StateSpace {
    const LABEL: &'static str = "StateId";
}

pub type EntryId<K> = RawId<EntrySpace, K>;
pub type StateId<K> = RawId<StateSpace, K>;
pub type AnyEntryId = AnyRawId<EntrySpace>;
pub type AnyStateId = AnyRawId<StateSpace>;

impl<P> AnyRawId<P> {
    pub const fn into_typed<K>(self) -> RawId<P, K> {
        RawId::from_raw(self.raw())
    }
}

impl<P, K> RawId<P, K> {
    pub const fn erase(self) -> AnyRawId<P> {
        AnyRawId::from_raw(self.raw())
    }

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw, PhantomData)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.raw() as usize
    }

    pub const fn next(self) -> Option<Self> {
        match self.raw().checked_add(1) {
            Some(x) => Some(Self::from_raw(x)),
            _ => None,
        }
    }

    pub const fn prev(self) -> Option<Self> {
        match self.raw().checked_sub(1) {
            Some(x) => Some(Self::from_raw(x)),
            _ => None,
        }
    }
}

impl<P, K> Clone for RawId<P, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, K> Copy for RawId<P, K> {}

impl<P, K> PartialEq for RawId<P, K> {
    fn eq(&self, other: &Self) -> bool {
        self.raw() == other.raw()
    }
}

impl<P, K> Eq for RawId<P, K> {}

impl<P, K> PartialOrd for RawId<P, K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<P, K> Ord for RawId<P, K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw().cmp(&other.raw())
    }
}

impl<P, K> Hash for RawId<P, K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw().hash(state)
    }
}

impl<P: IdSpace, K> Debug for RawId<P, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let kind = type_name::<K>().rsplit("::").next().unwrap_or("?");
        write!(f, "{}<{kind}>({})", P::LABEL, self.raw())
    }
}
