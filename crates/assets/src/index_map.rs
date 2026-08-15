use core::{hash::Hasher, ops::Index};

use alloc::vec::Vec;

use crate::{StableId, hash::FxHasher};

pub struct IndexMap<S> {
    entries: Vec<S>,
    table: Vec<(u64, u32)>,
}

impl<S> Default for IndexMap<S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            table: Vec::new(),
        }
    }
}

impl<S: StableId> IndexMap<S> {
    const SEED: usize = 0x517c_c1b7_2722_0a95_u64 as usize;
    fn mask(&self) -> usize {
        self.table.len() - 1
    }

    pub fn fixed_hash_of(stable_id: &S) -> u64 {
        let mut hasher = FxHasher::with_seed(Self::SEED);
        stable_id.hash(&mut hasher);
        hasher.finish()
    }

    pub fn find(&self, stable_id: &S) -> Option<u32> {
        self.find_by_hash(Self::fixed_hash_of(stable_id), stable_id)
    }

    pub fn find_by_hash(&self, hash: u64, stable_id: &S) -> Option<u32> {
        if self.table.is_empty() {
            return None;
        }
        let mask = self.mask();
        let mut i = hash as usize & mask;
        loop {
            let (h, entry) = self.table[i];
            if entry == u32::MAX {
                return None;
            }
            if h == hash && self.entries[entry as usize] == *stable_id {
                return Some(entry);
            }
            i = (i + 1) & mask;
        }
    }

    pub fn insert_by_hash_unchecked(&mut self, hash: u64, stable_id: S) -> u32 {
        if (self.entries.len() + 1) * 4 > self.table.len() * 3 {
            self.grow();
        }
        let mask = self.mask();
        let mut i = hash as usize & mask;
        while self.table[i].1 != u32::MAX {
            i = (i + 1) & mask;
        }

        let index = self.entries.len() as u32;
        self.table[i] = (hash, index);
        self.entries.push(stable_id);
        index
    }

    fn grow(&mut self) {
        let cap = (self.table.len() * 2).max(16);
        let old = core::mem::replace(&mut self.table, alloc::vec![(0, u32::MAX); cap]);
        let mask = cap - 1;

        for (hash, entry) in old {
            if entry == u32::MAX {
                continue;
            }
            let mut i = hash as usize & mask;
            while self.table[i].1 != u32::MAX {
                i = (i + 1) & mask;
            }
            self.table[i] = (hash, entry);
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &S> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl<S> Index<usize> for IndexMap<S> {
    type Output = S;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}
