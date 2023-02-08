use std::iter::Iterator;
pub struct FlatInterleave<U: IntoIterator> {
    iters: Vec<U::IntoIter>,
    finished: bool,
    current_index: usize,
}
pub trait FlatInterleaveTrait: Iterator
where
    Self::Item: IntoIterator,
    Self: Sized,
{
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
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    #[test]
    fn test_one() {
        let a = vec![vec![1, 2, 3]];
        let b = a.iter().flat_interleave().cloned().collect_vec();
        assert_eq!(b, vec![1, 2, 3]);
    }
    #[test]
    fn test_two() {
        let a = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let b = a.iter().flat_interleave().cloned().collect_vec();
        assert_eq!(b, vec![1, 4, 2, 5, 3, 6]);
    }
    #[test]
    fn test_zero() {
        let a: Vec<Vec<()>> = vec![vec![]];
        let b = a.iter().flat_interleave().cloned().collect_vec();
        assert_eq!(b, vec![]);
    }
    #[test]
    fn test_flat_interleave() {
        let a = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let b = a.iter().flat_interleave().cloned().collect_vec();
        assert_eq!(b, vec![1, 4, 7, 2, 5, 8, 3, 6, 9]);
    }

    #[test]
    fn test_flat_interleave_long() {
        let a = vec![
            vec![1, 2, 3, 99, 100, 200, 300],
            vec![4, 5, 6],
            vec![7, 8, 9, 10],
        ];
        let b = a.iter().flat_interleave().cloned().collect_vec();
        assert_eq!(b, vec![1, 4, 7, 2, 5, 8, 3, 6, 9, 99, 10, 100, 200, 300]);
    }

    #[test]
    fn test_nested() {
        let a = vec![
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![vec![7, 8, 9], vec![10, 11, 12]],
        ];
        let b = a
            .iter()
            .flat_interleave()
            .flat_interleave()
            .cloned()
            .collect_vec();
        assert_eq!(b, vec![1, 7, 4, 10, 2, 8, 5, 11, 3, 9, 6, 12]);
    }

    #[test]
    fn test_nested_long() {
        let a = vec![
            vec![vec![1, 2, 3, 99, 100, 200, 300], vec![4, 5, 6]],
            vec![vec![7, 8, 9, 10], vec![10, 11, 12]],
        ];
        let b = a
            .iter()
            .flat_interleave()
            .flat_interleave()
            .cloned()
            .collect_vec();
        assert_eq!(
            b,
            vec![1, 7, 4, 10, 2, 8, 5, 11, 3, 9, 6, 12, 99, 10, 100, 200, 300]
        );
    }
}
