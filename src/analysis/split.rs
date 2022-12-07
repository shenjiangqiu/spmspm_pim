//! this module is used to split the matrix into multiple partitions

use std::{
    fmt::{Debug, Display, Write},
    iter,
};

use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMat, CsMatView};
use tracing::debug;
fn write_matrix(f: &mut impl Write, matrix: CsMatView<Pattern>) -> std::fmt::Result {
    assert!(matrix.is_csr());
    for (row_id, row) in matrix.outer_iterator().enumerate() {
        writeln!(f, "row {}; cols: ", row_id)?;
        for col_id in row.iter().map(|(col_id, _)| col_id) {
            write!(f, " {} ", col_id)?;
        }
        writeln!(f)?;
    }
    Ok(())
}

/// the result matrix returned by [`split_matrix`]
pub struct SplitMatrix {
    pub matrix: Vec<CsMat<Pattern>>,
    pub start_points: Vec<usize>,
}
#[derive(Debug)]
pub struct NnzStats {
    pub mean: f64,
    pub max: usize,
    pub min: usize,
}
impl SplitMatrix {
    pub fn nnz_stats(&self) -> NnzStats {
        let nnz = self.matrix.iter().map(|m| m.nnz()).collect::<Vec<_>>();
        debug!("nnz: {:?}", nnz);
        let mean = nnz.iter().sum::<usize>() as f64 / nnz.len() as f64;
        let max = nnz.iter().max().unwrap();
        let min = nnz.iter().min().unwrap();
        NnzStats {
            mean,
            max: *max,
            min: *min,
        }
    }
}

impl Display for SplitMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SplitMatrix {{ matrix: [")?;
        for (i, m) in self.matrix.iter().enumerate() {
            writeln!(f, "matrix {} ", i)?;
            write_matrix(f, m.view())?;
        }
        writeln!(f, "], start_points: [")?;
        for (i, p) in self.start_points.iter().enumerate() {
            write!(f, "point {} ", i)?;
            write!(f, " {} ", p)?;
        }
        write!(f, "] }}")?;
        Ok(())
    }
}
impl Debug for SplitMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

/// split a matrix into multiple matrices by the given indices
/// this will spilt by columns
/// # Example
/// original matrix:
///
/// ```text
/// x x 1 x x x
/// 1 x x 1 x x
/// x 1 x x 1 x
/// 1 x 1 x 1 x
/// x x x x x x
/// ```
/// start points: `[0, 2, 4]`
///
/// result matrix:
/// ```text
/// x x | 1 x | x x
/// 1 x | x 1 | x x
/// x 1 | x x | 1 x
/// 1 x | 1 x | 1 x
/// x x | x x | x x
/// ```
///
/// - the result matrix and input result should be csr format
pub fn split_matrix(input_matrix: CsMat<Pattern>, start_points: Vec<usize>) -> SplitMatrix {
    // make sure it's csc
    let input_matrix = match input_matrix.storage() {
        sprs::CompressedStorage::CSR => input_matrix.to_csc(),
        sprs::CompressedStorage::CSC => input_matrix,
    };
    let cols = input_matrix.cols();
    let ranges = start_points.iter().chain(iter::once(&cols)).tuple_windows();
    let mut result_matrix = Vec::new();
    for (a, b) in ranges {
        // println!("{} {}", a, b);
        let csc_slice = input_matrix.slice_outer(*a..*b);
        let csr_slice = csc_slice.to_csr();
        result_matrix.push(csr_slice.to_owned());
    }
    SplitMatrix {
        matrix: result_matrix,
        start_points,
    }
}

#[cfg(test)]
mod tests {
    use sprs::{num_kinds::Pattern, CsMat, CsMatView};

    use super::*;
    fn print_matrix(matrix: CsMatView<Pattern>) {
        let mut s = String::new();
        write_matrix(&mut s, matrix).unwrap();
        println!("{}", s);
    }
    #[test]
    fn test_split() {
        let matrix: CsMat<Pattern> = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();

        print_matrix(matrix.view());
        let matrix_slice = matrix.slice_outer(2..5);
        print_matrix(matrix_slice);
    }

    #[test]
    fn test_split_col() {
        let matrix: CsMat<Pattern> = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();
        print_matrix(matrix.view());
        let start_points = vec![0, 2, 4];
        let SplitMatrix {
            matrix,
            start_points: _,
        } = split_matrix(matrix, start_points);
        for (i, matrix) in matrix.iter().enumerate() {
            println!("matrix {}", i);
            print_matrix(matrix.view());
        }
    }
}
