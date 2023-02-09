//! the flat_interleave trait allows you to interleave the elements of multiple iterators
//! in a way that is similar to the interleave trait, but it flattens the iterators first.
//! see [`FlatInterleaveTrait::flat_interleave`] for more details.

use std::iter::Iterator;

/// the iterator returned by [`FlatInterleaveTrait::flat_interleave`]

pub struct FlatInterleave<U: IntoIterator> {
    iters: Vec<U::IntoIter>,
    finished: bool,
    current_index: usize,
}

/// the trait that allows you to interleave the elements of multiple iterators
/// interleave the elements of multiple iterators
/// # Example
/// ```
/// use spmspm_pim::tools::FlatInterleaveTrait;
/// use itertools::Itertools;
/// fn test_two() {
///    let a = vec![vec![1, 2, 3], vec![4, 5, 6]];
///    let b = a.iter().flat_interleave().cloned().collect_vec();
///    assert_eq!(b, vec![1, 4, 2, 5, 3, 6]);
/// }
/// ```
pub trait FlatInterleaveTrait: Iterator
where
    Self::Item: IntoIterator,
    Self: Sized,
{
    /// the trait that allows you to interleave the elements of multiple iterators
    /// interleave the elements of multiple iterators
    /// # Example
    /// ```
    /// use spmspm_pim::tools::FlatInterleaveTrait;
    /// use itertools::Itertools;
    /// fn test_two() {
    ///    let a = vec![vec![1, 2, 3], vec![4, 5, 6]];
    ///    let b = a.iter().flat_interleave().cloned().collect_vec();
    ///    assert_eq!(b, vec![1, 4, 2, 5, 3, 6]);
    /// }
    /// ```
    fn flat_interleave(self) -> FlatInterleave<Self::Item> {
        FlatInterleave {
            iters: self.map(|x| x.into_iter()).collect(),
            finished: false,
            current_index: 0,
        }
    }
}
impl<T> FlatInterleaveTrait for T
where
    T: Iterator,
    T::Item: IntoIterator,
{
}

impl<U: IntoIterator> Iterator for FlatInterleave<U> {
    type Item = U::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let mut index = self.current_index;
        loop {
            if let Some(x) = self.iters[index].next() {
                index = (index + 1) % self.iters.len();

                self.current_index = index;
                return Some(x);
            }
            index = (index + 1) % self.iters.len();

            if index == self.current_index {
                self.finished = true;
                return None;
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut min = 0;
        let mut max = None;
        for iter in &self.iters {
            let (a, b) = iter.size_hint();
            min += a;
            max = match (max, b) {
                (Some(x), Some(y)) => Some(x + y),
                (Some(x), None) => Some(x),
                (None, Some(y)) => Some(y),
                (None, None) => None,
            };
        }
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    #[test]
    fn test_one() {
        let a = vec![vec![1, 2, 3]];
        let b = a.into_iter().flat_interleave();
        assert_eq!(b.size_hint(), (3, Some(3)));
        assert_eq!(b.collect_vec(), vec![1, 2, 3]);
    }
    #[test]
    fn test_two() {
        let a = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let b = a.into_iter().flat_interleave();
        assert_eq!(b.size_hint(), (6, Some(6)));
        assert_eq!(b.collect_vec(), vec![1, 4, 2, 5, 3, 6]);
    }
    #[test]
    fn test_zero() {
        let a: Vec<Vec<()>> = vec![vec![]];
        let b = a.into_iter().flat_interleave();
        assert_eq!(b.size_hint(), (0, Some(0)));
        assert_eq!(b.collect_vec(), vec![]);
    }
    #[test]
    fn test_flat_interleave() {
        let a = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let b = a.into_iter().flat_interleave();
        assert_eq!(b.size_hint(), (9, Some(9)));
        assert_eq!(b.collect_vec(), vec![1, 4, 7, 2, 5, 8, 3, 6, 9]);
    }

    #[test]
    fn test_flat_interleave_long() {
        let a = vec![
            vec![1, 2, 3, 99, 100, 200, 300],
            vec![4, 5, 6],
            vec![7, 8, 9, 10],
        ];
        let b = a.into_iter().flat_interleave();
        assert_eq!(b.size_hint(), (14, Some(14)));
        assert_eq!(
            b.collect_vec(),
            vec![1, 4, 7, 2, 5, 8, 3, 6, 9, 99, 10, 100, 200, 300]
        );
    }

    #[test]
    fn test_nested() {
        let a = vec![
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![vec![7, 8, 9], vec![10, 11, 12]],
        ];
        let b = a.into_iter().flat_interleave().flat_interleave();
        assert_eq!(b.size_hint(), (12, Some(12)));
        let b = b.collect_vec();
        assert_eq!(b, vec![1, 7, 4, 10, 2, 8, 5, 11, 3, 9, 6, 12]);
    }

    #[test]
    fn test_nested_long() {
        let a = vec![
            vec![vec![1, 2, 3, 99, 100, 200, 300], vec![4, 5, 6]],
            vec![vec![7, 8, 9, 10], vec![10, 11, 12]],
        ];
        let b = a.into_iter().flat_interleave().flat_interleave();
        assert_eq!(b.size_hint(), (17, Some(17)));
        let b = b.collect_vec();
        assert_eq!(
            b,
            vec![1, 7, 4, 10, 2, 8, 5, 11, 3, 9, 6, 12, 99, 10, 100, 200, 300]
        );
    }
}
