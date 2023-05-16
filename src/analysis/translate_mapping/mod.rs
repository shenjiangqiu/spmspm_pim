pub mod evil_mapping;
pub mod weighted;
use crate::analysis::remap_analyze::row_cycle::*;
use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMatI, CsMatViewI, CsVecViewI, TriMatI};
use tracing::debug;
pub mod same_bank;

/// the mapping that translate the original
pub trait TranslateMapping {
    type RowSubMapping: RowSubarrayMapping;
    fn get_row_sub_mapping(&self) -> &Self::RowSubMapping;
    fn get_location(&self, row_id: LogicRowId, graph: CsMatViewI<Pattern, u32>) -> RowLocation {
        self.get_row_sub_mapping().get_location(row_id, graph)
    }
    fn get_location_evil<'a>(
        &'a self,
        row_id: LogicRowId,
        graph: CsMatViewI<Pattern, u32>,
    ) -> Vec<(SubarrayId, RowLocation, CsVecViewI<'a, Pattern, u32>)> {
        self.get_row_sub_mapping().get_location_evil(row_id, graph)
    }
    fn get_dense_location(
        &self,
        target_row_id: LogicRowId,
        col_id: LogicColId,
        graph: CsMatViewI<Pattern, u32>,
    ) -> RowLocation {
        self.get_row_sub_mapping()
            .get_dense_location(target_row_id, col_id, graph)
    }
    fn is_evil(&self, row_id: LogicRowId) -> bool;
}

/// this trait is a mapping which get the logic location to physic location
pub trait RowSubarrayMapping {
    /// get the physic location of a matrix b row
    fn get_location(&self, row_id: LogicRowId, graph: CsMatViewI<Pattern, u32>) -> RowLocation;
    /// get the physic locations of a evil matrix b row.
    fn get_location_evil<'a>(
        &'a self,
        row_id: LogicRowId,
        graph: CsMatViewI<Pattern, u32>,
    ) -> Vec<(SubarrayId, RowLocation, CsVecViewI<'a, Pattern, u32>)>;
    /// get location of the matrix b dense column
    fn get_dense_location(
        &self,
        target_row_id: LogicRowId,
        col_id: LogicColId,
        graph: CsMatViewI<Pattern, u32>,
    ) -> RowLocation;
}

pub fn get_partition_id(id: usize, bounds: &UpperLowerBound) -> usize {
    if id < bounds.upper_bound_rows {
        id / bounds.average_row_per_partition_upper_bound
    } else {
        let lower_id = id - bounds.upper_bound_rows;
        let lower_patition = lower_id / bounds.average_row_per_partition_lower_bound;
        lower_patition + bounds.upper_bound_partitions
    }
}
#[derive(Debug)]
pub struct RowAverageMapping {
    evil_rows: usize,
    upper_lower_bounds: UpperLowerBound,
    non_evil_accumulated_nnz: Vec<usize>,
    evil_row_accumulated_nnz: Vec<Vec<usize>>,
    evil_row_sub_graph: Vec<CsMatI<Pattern, u32>>,
}
#[derive(Debug)]
pub struct AverageMapping {
    cols: usize,
    row_mapping: RowAverageMapping,
    col_mapping: UpperLowerBound,
}
impl AverageMapping {
    pub fn new(
        graph: CsMatViewI<Pattern, u32>,
        evil_rows: usize,
        subarrays: usize,
        cols: usize,
    ) -> Self {
        // first calculate the accumulated nnz of the graph
        let non_evil_accumulated_nnz =
            graph
                .outer_iterator()
                .skip(evil_rows)
                .fold(vec![0], |mut acc, row| {
                    let last = acc.last().unwrap();
                    acc.push(last + row.nnz());
                    acc
                });
        let non_evil_rows = graph.rows() - evil_rows;
        debug!(non_evil_rows);

        let row_bounds = get_upper_lower_bound(non_evil_rows, subarrays);
        let col_bounds = get_upper_lower_bound(graph.rows(), subarrays);

        // then handle the evil row
        // for each evil row, we biuld a subgraph for it
        // fix the bug here, the row if the partial graph is evil_rows, otherwise the size will be too large!(2000GB+)
        let mut evil_row_sub_graph: Vec<TriMatI<Pattern, u32>> = (0..subarrays)
            .map(|_| TriMatI::new((evil_rows, graph.cols())))
            .collect();
        for (row_id, evil_row) in graph.outer_iterator().enumerate().take(evil_rows) {
            for col_id in evil_row.indices() {
                let partition_id = get_partition_id(*col_id as usize, &col_bounds);
                evil_row_sub_graph[partition_id].add_triplet(row_id, *col_id as usize, Pattern);
            }
        }
        let evil_row_sub_graph: Vec<CsMatI<Pattern, u32>> = evil_row_sub_graph
            .into_iter()
            .map(|x| x.to_csr())
            .collect_vec();
        let evil_row_accumulated_nnz = evil_row_sub_graph
            .iter()
            .map(|x| {
                x.outer_iterator().fold(vec![0], |mut acc, row| {
                    let last = acc.last().unwrap();
                    acc.push(last + row.nnz());
                    acc
                })
            })
            .collect_vec();

        let row_mapping = RowAverageMapping {
            evil_rows,
            upper_lower_bounds: row_bounds,
            non_evil_accumulated_nnz,
            evil_row_accumulated_nnz,
            evil_row_sub_graph,
        };

        Self {
            cols,
            row_mapping,
            col_mapping: col_bounds,
        }
    }
}
#[derive(Debug)]
pub struct UpperLowerBound {
    pub average_row_per_partition_lower_bound: usize,
    pub upper_bound_partitions: usize,
    pub upper_bound_rows: usize,
    pub average_row_per_partition_upper_bound: usize,
}
/// get the upper and lower bound of the average row per partition
/// # Return
/// (average_row_per_partition_lower_bound, upper_bound_partitions, upper_bound_rows, average_row_per_partition_upper_bound)
fn get_upper_lower_bound(rows: usize, subarrays: usize) -> UpperLowerBound {
    let average_row_per_partition_lower_bound = rows / subarrays;

    let upper_bound_partitions = rows - average_row_per_partition_lower_bound * subarrays;
    let upper_bound_rows = upper_bound_partitions * (average_row_per_partition_lower_bound + 1);
    let lower_bound_rows =
        average_row_per_partition_lower_bound * (subarrays - upper_bound_partitions);

    assert_eq!(upper_bound_rows + lower_bound_rows, rows);

    let average_row_per_partition_upper_bound = average_row_per_partition_lower_bound + 1;
    UpperLowerBound {
        average_row_per_partition_lower_bound,
        upper_bound_partitions,
        upper_bound_rows,
        average_row_per_partition_upper_bound,
    }
}

impl RowSubarrayMapping for AverageMapping {
    fn get_location(&self, row_id: LogicRowId, _graph: CsMatViewI<Pattern, u32>) -> RowLocation {
        let row_id = row_id.0;
        assert!(
            row_id >= self.row_mapping.evil_rows,
            "evil row should not go there, try get_location_evil"
        );
        let row_mapping = &self.row_mapping;
        let row_bounds = &row_mapping.upper_lower_bounds;
        let non_evil_row_id = row_id - row_mapping.evil_rows;
        if non_evil_row_id < row_bounds.upper_bound_rows {
            let subarray_id = non_evil_row_id / row_bounds.average_row_per_partition_upper_bound;
            let start_row_id = subarray_id * row_bounds.average_row_per_partition_upper_bound;
            let flat_nnz = row_mapping.non_evil_accumulated_nnz[non_evil_row_id]
                - row_mapping.non_evil_accumulated_nnz[start_row_id];
            let flat_col_id = flat_nnz * 8;
            let physic_row_id = PhysicRowId::new(flat_col_id / self.cols);
            let physic_col_id = PhysicColId::new(flat_col_id % self.cols);
            let word_id = physic_col_id.word_id();
            assert!(word_id.0 < 64);
            debug!(
                target : "location",
                ?row_id,
                ?subarray_id,
                ?physic_row_id,
                ?physic_col_id,
                ?word_id,
                "get location, cols: {}",
                self.cols
            );
            RowLocation::new(
                SubarrayId(subarray_id),
                RowIdWordId::new(physic_row_id, word_id),
            )
        } else {
            // it's the low bound
            let lower_id = non_evil_row_id - row_bounds.upper_bound_rows;
            let lower_patition = lower_id / row_bounds.average_row_per_partition_lower_bound;

            let subarray_id = lower_patition + row_bounds.upper_bound_partitions;
            let start_row_id = row_bounds.upper_bound_rows
                + lower_patition * row_bounds.average_row_per_partition_lower_bound;
            let flat_nnz = row_mapping.non_evil_accumulated_nnz[non_evil_row_id]
                - row_mapping.non_evil_accumulated_nnz[start_row_id];
            let flat_col_id = flat_nnz * 8;
            let physic_row_id = PhysicRowId::new(flat_col_id / self.cols);
            let physic_col_id = PhysicColId::new(flat_col_id % self.cols);
            let word_id = physic_col_id.word_id();
            debug!(
                target : "location",
                ?row_id,
                ?subarray_id,
                ?physic_row_id,
                ?physic_col_id,
                ?word_id,
                "get location, cols: {}",
                self.cols
            );
            RowLocation::new(
                SubarrayId(subarray_id),
                RowIdWordId::new(physic_row_id, word_id),
            )
        }
    }

    fn get_location_evil<'a>(
        &'a self,
        row_id: LogicRowId,
        _graph: CsMatViewI<Pattern, u32>,
    ) -> Vec<(SubarrayId, RowLocation, CsVecViewI<'a, Pattern, u32>)> {
        let row_id = row_id.0;
        let row_mapping = &self.row_mapping;
        assert!(
            row_id < row_mapping.evil_rows,
            "non evil row should not go there, try get_location"
        );
        let mut result = vec![];
        for (partition_id, graph) in row_mapping.evil_row_sub_graph.iter().enumerate() {
            let row = graph.outer_view(row_id).unwrap();
            if row.nnz() != 0 {
                let flat_col_id = row_mapping.evil_row_accumulated_nnz[partition_id][row_id] * 8;
                let physic_row_id = PhysicRowId::new(flat_col_id / self.cols);
                let physic_col_id = PhysicColId::new(flat_col_id % self.cols);
                let word_id = physic_col_id.word_id();
                result.push((
                    SubarrayId::new(partition_id),
                    RowLocation::new(
                        SubarrayId(partition_id),
                        RowIdWordId::new(physic_row_id, word_id),
                    ),
                    row,
                ));
            }
        }
        debug!(
            target : "location",
            ?row_id,
            "get location, cols: {}",
            self.cols
        );
        result
    }

    fn get_dense_location(
        &self,
        _target_row_id: LogicRowId,
        col_id: LogicColId,
        _graph: CsMatViewI<Pattern, u32>,
    ) -> RowLocation {
        let col_mapping = &self.col_mapping;

        if col_id.0 < col_mapping.upper_bound_rows {
            let subarray_id = col_id.0 / col_mapping.average_row_per_partition_upper_bound;
            let start_row_id = subarray_id * col_mapping.average_row_per_partition_upper_bound;
            let flat_col_id = col_id.0 - start_row_id;
            let flat_col_id = flat_col_id * 4;
            let physic_row_id = PhysicRowId::new(flat_col_id / self.cols);
            let physic_col_id = PhysicColId::new(flat_col_id % self.cols);
            let word_id = physic_col_id.word_id();
            debug!(
                target : "location",
                ?col_id,
                ?subarray_id,
                ?physic_row_id,
                ?physic_col_id,
                ?word_id,
                "get location, cols: {}",
                self.cols
            );
            RowLocation::new(
                SubarrayId(subarray_id),
                RowIdWordId::new(physic_row_id, word_id),
            )
        } else {
            // it's the low bound
            let lower_id = col_id.0 - col_mapping.upper_bound_rows;
            let lower_patition = lower_id / col_mapping.average_row_per_partition_lower_bound;

            let subarray_id = lower_patition + col_mapping.upper_bound_partitions;
            let start_row_id = col_mapping.upper_bound_rows
                + lower_patition * col_mapping.average_row_per_partition_lower_bound;
            let flat_col_id = col_id.0 - start_row_id;
            let flat_col_id = flat_col_id * 4;
            let physic_row_id = PhysicRowId::new(flat_col_id / self.cols);
            let physic_col_id = PhysicColId::new(flat_col_id % self.cols);
            let word_id = physic_col_id.word_id();
            debug!(
                target : "location",
                ?col_id,
                ?subarray_id,
                ?physic_row_id,
                ?physic_col_id,
                ?word_id,
                "get location, cols: {}",
                self.cols
            );
            RowLocation::new(
                SubarrayId(subarray_id),
                RowIdWordId::new(physic_row_id, word_id),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use sprs::{num_kinds::Pattern, TriMatI};

    use crate::tools;

    use super::{AverageMapping, RowSubarrayMapping};

    #[test]
    fn test_average_mapping() {
        let graph: TriMatI<Pattern, u32> = sprs::io::read_matrix_market_from_bufread(
            &mut tools::file_server::file_reader("test_mtx/test_large.mtx").unwrap(),
        )
        .unwrap();
        let evil_rows = 2;
        let graph_csr = graph.to_csr();
        for row in graph_csr.outer_iterator() {
            println!("{:?}", row);
        }
        let average_mapping = AverageMapping::new(graph_csr.view(), evil_rows, 4, 16);
        let total_rows = graph_csr.rows();
        (evil_rows..total_rows)
            .map(|row| {
                println!("row: {}", row);
                average_mapping.get_location(row.into(), graph_csr.view())
            })
            .group_by(|loc| loc.subarray_id)
            .into_iter()
            .for_each(|g| {
                println!("subarray_id: {}", g.0);
                for loc in g.1 {
                    println!("{:?}", loc);
                }
            });

        for row in 0..evil_rows {
            let evil_location = average_mapping.get_location_evil(row.into(), graph_csr.view());
            println!("{:?}", evil_location);
        }
        for col in 0..total_rows {
            let dense_location =
                average_mapping.get_dense_location(0.into(), col.into(), graph_csr.view());
            println!("{:?}", dense_location);
        }
    }
}
