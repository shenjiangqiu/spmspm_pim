// //! the original gearbox implementation: one row product an matrix, we choose the first row of matrix a as input mvec
// //!  and the whole matrix b as input matrix.

// use hashbrown::HashSet;
// use itertools::Itertools;
// use sprs::{num_kinds::Pattern, CsMat, TriMat};
// use tracing::info;

// use super::analyze_gearbox::{GearboxResult, SingleResult};
// use crate::pim::{
//     config::Config,
//     level::{ddr4, LevelTrait},
// };
// use std::{cmp::Reverse, fmt::Debug, path::Path};

// /// analyze the split spmm
// pub(crate) fn analyze_gearbox(config: &Config) -> GearboxResult {
//     match config.dram_type {
//         crate::pim::config::DramType::DDR3 => unimplemented!(),
//         crate::pim::config::DramType::DDR4 => {
//             let total_size = ddr4::Storage::new(
//                 config.channels.num,
//                 config.ranks.num,
//                 config.chips.num,
//                 config.bank_groups.num,
//                 config.banks.num,
//                 config.subarrays,
//                 config.rows,
//                 config.columns,
//             );
//             analyze_gearbox_inner::<ddr4::Level>(config, &total_size)
//         }
//         crate::pim::config::DramType::LPDDR3 => unimplemented!(),
//         crate::pim::config::DramType::LPDDR4 => unimplemented!(),
//         crate::pim::config::DramType::HBM => unimplemented!(),
//         crate::pim::config::DramType::HBM2 => unimplemented!(),
//     }
// }

// fn analyze_gearbox_inner<LevelType: LevelTrait>(
//     config: &Config,
//     _total_size: &LevelType::Storage,
// ) -> GearboxResult
// where
//     LevelType::Storage: Debug + Sync,
//     LevelType::Mapping: Debug,
// {
//     let total_graphs = config.graph_path.len();
//     let results = config
//         .graph_path
//         .iter()
//         .enumerate()
//         .map(|(index, path)| {
//             info!("analyzing graph {}/{}", index + 1, total_graphs);
//             compute_gearbox(config, path)
//         })
//         .collect_vec();

//     GearboxResult { results }
// }

// fn compute_gearbox(config: &Config, path: &str) -> SingleResult {
//     let partitions = config.channels.num
//         * config.ranks.num
//         * config.chips.num
//         * config.bank_groups.num
//         * config.banks.num
//         * config.subarrays;
//     info!(?partitions, "compute gearbox");
//     info!("reading mtx file: {}", path);
//     let matrix_a: TriMat<Pattern> = sprs::io::read_matrix_market(path).unwrap();
//     let (matrix_a, matrix_b): (CsMat<Pattern>, CsMat<Pattern>) =
//         (matrix_a.to_csr(), matrix_a.transpose_view().to_csr());
//     let mat_b_rows = matrix_b.rows();
//     let mat_b_cols = matrix_b.cols();

//     info!(?mat_b_rows, ?mat_b_cols, "matrix b shape");

//     // the nnz of matrix b rows
//     let mut mat_b_row_ids = (0..mat_b_rows)
//         .zip(matrix_b.outer_iterator().map(|row| row.nnz()))
//         .collect_vec();
//     // the nnz of matrix b cols
//     let mut mat_b_col_ids = (0..mat_b_cols)
//         .zip(matrix_a.outer_iterator().map(|row| row.nnz()))
//         .collect_vec();
//     mat_b_row_ids.sort_by_key(|(_index, nnz)| Reverse(*nnz));
//     mat_b_col_ids.sort_by_key(|(_index, nnz)| Reverse(*nnz));

//     let top_rows = (mat_b_row_ids.len() as f32 * config.gearbox_config.topk) as usize;
//     let top_rows = if top_rows == 0 { 1 } else { top_rows };
//     info!(?top_rows, "top rows");
//     let top_cols = (mat_b_col_ids.len() as f32 * config.gearbox_config.topk) as usize;
//     let top_cols = if top_cols == 0 { 1 } else { top_cols };
//     info!(?top_cols, "top cols");
//     assert!(top_cols > 0);
//     info!(
//         "the top 10 rows: {:?}",
//         mat_b_row_ids.iter().take(10).collect_vec()
//     );
//     info!(
//         "the top 10 cols: {:?}",
//         mat_b_col_ids.iter().take(10).collect_vec()
//     );

//     let mut gearbox = GearboxSim::new(
//         partitions,
//         mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
//         mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
//         matrix_b,
//         config,
//     );

//     gearbox.run(&matrix_a);
//     gearbox.report(path.to_string())
// }
// pub struct GearboxSim {
//     pub row_per_partition: usize,
//     #[allow(unused)]
//     pub col_per_partition: usize,
//     /// the id of evil col: this col will  have a backup copy in each partition
//     pub evil_col_ids: HashSet<usize>,
//     /// the id of evil row: the evil row will be partitioned into each components,there are no remote access needed.
//     pub evil_row_ids: HashSet<usize>,
//     pub matrix_b: CsMat<Pattern>,
//     pub hardware: Hardware,
// }
// struct Hardware {
//     sub_array: Vec<SubArray>,
//     ring: Vec<Ring>,
//     tsv: Vec<Tsv>,
//     config: Config,
//     /// the dimension of dense matrix in one subarray
//     dense_dim: usize,
//     /// for normal rows, distribute them to different subarrays
//     row_per_partition: usize,
//     /// for target rows, distribute the cols to different subarrays
//     /// and for the evil row, distribute them by column
//     col_per_partition: usize,
// }
