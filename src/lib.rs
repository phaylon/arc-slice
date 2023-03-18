use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use arrayvec::ArrayVec;


pub trait ArcSliceSplit: Sized {
    type Item;

    fn arc_slice_split_first(&self) -> Option<(&Self::Item, Self)>;
    fn arc_slice_split_last(&self) -> Option<(&Self::Item, Self)>;
}

pub struct ArcSlice<T> {
    inner: ArcSliceInner<T>,
}

impl<T> ArcSlice<T> {
    fn raw_inner_slice(&self) -> &[T] {
        match &self.inner {
            ArcSliceInner::Empty => &[],
            ArcSliceInner::Shared(slice, range) => &slice[range.clone()],
        }
    }
}

impl<T> ArcSliceSplit for ArcSlice<T> {
    type Item = T;

    fn arc_slice_split_first(&self) -> Option<(&Self::Item, Self)> {
        let ArcSliceInner::Shared(slice, range) = &self.inner else {
            return None;
        };
        if range.start < range.end {
            Some((&slice[range.clone()][0], Self {
                inner: ArcSliceInner::Shared(slice.clone(), (range.start + 1)..range.end),
            }))
        } else {
            None
        }
    }

    fn arc_slice_split_last(&self) -> Option<(&Self::Item, Self)> {
        let ArcSliceInner::Shared(slice, range) = &self.inner else {
            return None;
        };
        if range.start < range.end {
            Some((slice[range.clone()].last().unwrap(), Self {
                inner: ArcSliceInner::Shared(slice.clone(), range.start..(range.end - 1)),
            }))
        } else {
            None
        }
    }
}

impl<T, const N: usize> From<[T; N]> for ArcSlice<T> {
    fn from(values: [T; N]) -> Self {
        if N == 0 {
            Self { inner: ArcSliceInner::Empty }
        } else {
            let slice = Arc::from(values);
            let range = 0..N;
            Self { inner: ArcSliceInner::Shared(slice, range) }
        }
    }
}

impl<T> FromIterator<T> for ArcSlice<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        if let Some(first) = iter.next() {
            let slice = Arc::from_iter([first].into_iter().chain(iter));
            let range = 0..slice.len();
            Self { inner: ArcSliceInner::Shared(slice, range) }
        } else {
            Self { inner: ArcSliceInner::Empty }
        }
    }
}

impl<T> Default for ArcSlice<T> {
    fn default() -> Self {
        Self {
            inner: ArcSliceInner::Empty,
        }
    }
}

impl<T: std::hash::Hash> std::hash::Hash for ArcSlice<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw_inner_slice().hash(state)
    }
}

impl<T: Ord> Ord for ArcSlice<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw_inner_slice().cmp(other.raw_inner_slice())
    }
}

impl<T: PartialOrd> PartialOrd for ArcSlice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.raw_inner_slice().partial_cmp(other.raw_inner_slice())
    }
}

impl<T: Eq> Eq for ArcSlice<T> {}

impl<T: PartialEq> PartialEq for ArcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_inner_slice() == other.raw_inner_slice()
    }
}

impl<T> Clone for ArcSlice<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std::ops::Deref for ArcSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.raw_inner_slice()
    }
}

impl<T: fmt::Debug> fmt::Debug for ArcSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw_inner_slice().fmt(f)
    }
}

impl<T> IntoIterator for ArcSlice<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = ArcSliceIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        ArcSliceIter { slice: self }
    }
}

enum ArcSliceInner<T> {
    Empty,
    Shared(Arc<[T]>, Range<usize>),
}

impl<T> Clone for ArcSliceInner<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Shared(arc, range) => Self::Shared(arc.clone(), range.clone()),
        }
    }
}

#[derive(Clone)]
pub struct SmallArcSlice<T, const CAP: usize> {
    inner: SmallArcSliceInner<T, CAP>,
    range: Range<usize>,
}

impl<T, const CAP: usize> SmallArcSlice<T, CAP> {
    fn raw_inner_slice(&self) -> &[T] {
        match &self.inner {
            SmallArcSliceInner::Inline(slice) => &slice[self.range.clone()],
            SmallArcSliceInner::Shared(slice) => &slice[self.range.clone()],
        }
    }
}

impl<T, const CAP: usize> ArcSliceSplit for SmallArcSlice<T, CAP>
where
    T: Clone,
{
    type Item = T;

    fn arc_slice_split_first(&self) -> Option<(&Self::Item, Self)> {
        if self.range.start < self.range.end {
            Some((&self.raw_inner_slice()[0], Self {
                inner: self.inner.clone(),
                range: (self.range.start + 1)..self.range.end,
            }))
        } else {
            None
        }
    }

    fn arc_slice_split_last(&self) -> Option<(&Self::Item, Self)> {
        if self.range.start < self.range.end {
            Some((self.raw_inner_slice().last().unwrap(), Self {
                inner: self.inner.clone(),
                range: self.range.start..(self.range.end - 1),
            }))
        } else {
            None
        }
    }
}

impl<T, const CAP: usize, const N: usize> From<[T; N]> for SmallArcSlice<T, CAP> {
    fn from(values: [T; N]) -> Self {
        let range = 0..N;
        if N <= CAP {
            Self {
                inner: SmallArcSliceInner::Inline(values.into_iter().collect()),
                range,
            }
        } else {
            Self {
                inner: SmallArcSliceInner::Shared(values.into_iter().collect()),
                range,
            }
        }
    }
}

impl<T, const CAP: usize> FromIterator<T> for SmallArcSlice<T, CAP> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = ArrayVec::new();
        let mut iter = iter.into_iter();
        while let Some(value) = iter.next() {
            if let Err(error) = array.try_push(value) {
                let value = error.element();
                let arc = Arc::from_iter(array.into_iter().chain([value]).chain(iter));
                let range = 0..arc.len();
                return Self {
                    inner: SmallArcSliceInner::Shared(arc),
                    range,
                };
            }
        }
        let range = 0..array.len();
        Self {
            inner: SmallArcSliceInner::Inline(array),
            range,
        }
    }
}

impl<T, const CAP: usize> Default for SmallArcSlice<T, CAP> {
    fn default() -> Self {
        Self {
            inner: SmallArcSliceInner::Inline(ArrayVec::new()),
            range: 0..0,
        }
    }
}

impl<T: std::hash::Hash, const CAP: usize> std::hash::Hash for SmallArcSlice<T, CAP> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw_inner_slice().hash(state);
    }
}

impl<T: Ord, const CAP: usize> Ord for SmallArcSlice<T, CAP> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw_inner_slice().cmp(other.raw_inner_slice())
    }
}

impl<T: PartialOrd, const CAP1: usize, const CAP2: usize> PartialOrd<SmallArcSlice<T, CAP2>>
for SmallArcSlice<T, CAP1> {
    fn partial_cmp(&self, other: &SmallArcSlice<T, CAP2>) -> Option<std::cmp::Ordering> {
        self.raw_inner_slice().partial_cmp(other.raw_inner_slice())
    }
}

impl<T: Eq, const CAP: usize> Eq for SmallArcSlice<T, CAP> {}

impl<T: PartialEq, const CAP1: usize, const CAP2: usize> PartialEq<SmallArcSlice<T, CAP2>>
for SmallArcSlice<T, CAP1> {
    fn eq(&self, other: &SmallArcSlice<T, CAP2>) -> bool {
        self.raw_inner_slice() == other.raw_inner_slice()
    }
}

impl<T, const CAP: usize> std::ops::Deref for SmallArcSlice<T, CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.raw_inner_slice()
    }
}

impl<T: fmt::Debug, const CAP: usize> fmt::Debug for SmallArcSlice<T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw_inner_slice().fmt(f)
    }
}

impl<T, const CAP: usize> IntoIterator for SmallArcSlice<T, CAP>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = ArcSliceIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        ArcSliceIter { slice: self }
    }
}

#[derive(Clone)]
enum SmallArcSliceInner<T, const CAP: usize> {
    Inline(ArrayVec<T, CAP>),
    Shared(Arc<[T]>),
}

#[derive(Clone)]
pub struct ArcSliceIter<T> {
    slice: T,
}

impl<I> Iterator for ArcSliceIter<I>
where
    I: ArcSliceSplit,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((next, rest)) = self.slice.arc_slice_split_first() {
            let next = next.clone();
            self.slice = rest;
            Some(next)
        } else {
            None
        }
    }
}
