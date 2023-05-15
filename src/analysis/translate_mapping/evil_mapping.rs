use std::{cmp::Reverse, collections::BinaryHeap};

use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMatI};
use tracing::debug;

pub fn build_evil_row_mapping(graph: &CsMatI<Pattern, u32>, evil_threshold: usize) -> Vec<usize> {
    debug!("start to build the evil ids");
    let mut top_rows = BinaryHeap::new();
    // step 1, select the evil rows
    for (row_id, row) in graph.outer_iterator().enumerate() {
        let nnz = row.nnz();
        if top_rows.len() < evil_threshold {
            top_rows.push(Reverse((nnz, row_id)));
        } else if nnz > top_rows.peek().unwrap().0 .0 {
            top_rows.pop();
            top_rows.push(Reverse((nnz, row_id)));
        }
    }
    assert_eq!(top_rows.len(), evil_threshold);
    debug!("finish building the evil ids, ids: {:?}", top_rows);
    let mut initial_mapping = (0..graph.rows()).collect_vec();
    let mut evil_rows = top_rows.into_iter().map(|x| x.0 .1).collect_vec();
    evil_rows.sort_unstable();
    for (new_id, row) in evil_rows.into_iter().enumerate() {
        initial_mapping.swap(new_id, row);
    }
    initial_mapping
}
