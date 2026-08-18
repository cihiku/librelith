use core::{cmp::Ordering, marker::PhantomData};

pub trait StableKey: Ord {
    type SortKey<'a>: AsRef<[u8]>
    where
        Self: 'a;
    fn sort_key(&self) -> Self::SortKey<'_>;
}

pub struct KeyTable<'a, K> {
    offsets: &'a [u32],
    bytes: &'a [u8],
    _space: PhantomData<fn() -> K>,
}

impl<'a, P> KeyTable<'a, P> {
    pub fn new(offsets: &'a [u32], bytes: &'a [u8]) -> Option<Self> {
        if *offsets.first()? != 0
            || offsets.windows(2).any(|w| w[0] > w[1])
            || *offsets.last()? as usize != bytes.len()
        {
            return None;
        }
        Some(Self {
            offsets,
            bytes,
            _space: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, i: u32) -> Option<&'a [u8]> {
        let start = *self.offsets.get(i as usize)? as usize;
        let end = *self.offsets.get(i as usize + 1)? as usize;
        self.bytes.get(start..end)
    }

    pub fn find(&self, key: &[u8]) -> Option<u32> {
        let (mut lo, mut hi) = (0u32, self.len() as u32);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.get(mid)?.cmp(key) {
                Ordering::Equal => return Some(mid),
                Ordering::Greater => hi = mid,
                Ordering::Less => lo = mid + 1,
            }
        }
        None
    }

    pub fn is_sorted(&self) -> bool {
        (1..self.len() as u32).all(|i| self.get(i - 1) < self.get(i))
    }
}
impl<'a, P> Clone for KeyTable<'a, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, P> Copy for KeyTable<'a, P> {}
