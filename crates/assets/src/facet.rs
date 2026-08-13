use core::{
    iter::{Enumerate, Zip},
    marker::PhantomData,
    ops::Index,
    slice::Iter,
};

use alloc::vec::Vec;

use crate::id::Id;

pub enum Storage<C> {
    Sparse,
    Dense(fn() -> C),
    Required,
}

/// ECS like component
pub trait Facet: Sized + Send + Sync + 'static {
    const STORAGE: Storage<Self> = Storage::Sparse;
}

#[derive(Debug)]
pub struct Column<K, C: Facet> {
    repr: Repr<C>,
    _k: PhantomData<fn() -> K>,
}

#[derive(Debug)]
enum Repr<C> {
    Dense(Vec<C>),
    Sparse { ids: Vec<u32>, values: Vec<C> },
}

impl<K, C: Facet> Column<K, C> {
    pub(crate) fn dense(values: Vec<C>) -> Self {
        Self {
            repr: Repr::Dense(values),
            _k: PhantomData,
        }
    }

    pub(crate) fn sparse(ids: Vec<u32>, values: Vec<C>) -> Self {
        Self {
            repr: Repr::Sparse { ids, values },
            _k: PhantomData,
        }
    }

    pub fn get(&self, id: Id<K>) -> Option<&C> {
        match &self.repr {
            Repr::Dense(values) => values.get(id.index()),
            Repr::Sparse { ids, values } => ids.binary_search(&id.raw()).ok().map(|i| &values[i]),
        }
    }

    pub fn as_slice(&self) -> Option<&[C]> {
        match &self.repr {
            Repr::Dense(values) => Some(values),
            Repr::Sparse { .. } => None,
        }
    }

    pub fn len(&self) -> usize {
        match &self.repr {
            Repr::Dense(values) => values,
            Repr::Sparse { values, .. } => values,
        }
        .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> ColumnIter<'_, K, C> {
        ColumnIter {
            repr: match &self.repr {
                Repr::Dense(values) => IterRepr::Dense(values.iter().enumerate()),
                Repr::Sparse { ids, values } => IterRepr::Sparse(ids.iter().zip(values)),
            },
            _k: PhantomData,
        }
    }
}

impl<K, C: Facet> Index<Id<K>> for Column<K, C> {
    type Output = C;
    fn index(&self, id: Id<K>) -> &C {
        self.get(id).expect("no value for id in column")
    }
}

pub struct ColumnIter<'a, K, C> {
    repr: IterRepr<'a, C>,
    _k: PhantomData<fn() -> K>,
}

enum IterRepr<'a, C> {
    Dense(Enumerate<Iter<'a, C>>),
    Sparse(Zip<Iter<'a, u32>, Iter<'a, C>>),
}

impl<'a, K, C> Iterator for ColumnIter<'a, K, C> {
    type Item = (Id<K>, &'a C);
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.repr {
            IterRepr::Dense(it) => it.next().map(|(i, c)| (Id::from_raw(i as u32), c)),
            IterRepr::Sparse(it) => it.next().map(|(&i, c)| (Id::from_raw(i), c)),
        }
    }
}
