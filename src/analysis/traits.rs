use std::{cmp::Reverse, collections::BTreeMap, sync::atomic::Ordering};

use crate::analysis::remap_analyze::row_cycle::*;
use crate::{
    analysis::{
        mapping::{
            same_bank::{self, SameBankMapping},
            same_bank_weighted::SameBankWeightedMapping,
            same_subarray::{self, SameSubarrayMapping},
        },
        TOTAL_FINISHED_TASKS, TOTAL_TASKS,
    },
    pim::{
        configv2::{ConfigV2, DramType},
        level::{ddr4, LevelTrait},
    },
    tools::stop_signal,
    TIME_TO_LOG,
};
use itertools::Itertools;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use sprs::{io::MatrixHead, num_kinds::Pattern, CsMatI, TriMatI};
use statrs::statistics::Statistics;
use tracing::{debug, info};
/// give an array of data, return each filed's mean, std, max
/// # Example
/// ```
/// use std::collections::BTreeMap;
/// use statrs::statistics::Statistics;
/// use spmspm_pim::analysis::traits::{ReportStats, get_mean_std_max};
///
/// #[derive(Debug)]
/// struct Data {
///   a: usize,
///   b: usize,
/// }
/// impl ReportStats for Data {
///    fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)> {  
///       let mut res = BTreeMap::new();    
///       res.insert("a".to_string(), get_mean_std_max(data_vec, |x| x.a));
///       res.insert("b".to_string(), get_mean_std_max(data_vec, |x| x.b));
///       res
///  }
/// }
/// let data = vec![Data { a: 1, b: 2 }, Data { a: 2, b: 3 }];
/// let res = Data::report_stats(&data);
/// assert_eq!(res["a"], (1.5, 0.5, 2));
/// assert_eq!(res["b"], (2.5, 0.5, 3));
/// ```
///
pub trait ReportStats: Sized {
    fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)>;
}
use core::fmt::Debug;
pub enum DistributeType {
    EvilRow,
    EvilCol,
    Local,
    Remote,
}
use super::mapping::Mapping;
#[allow(unused_variables)]
pub trait GearboxSimTrait<'matrix, 'config> {
    type Mapping: Mapping;
    type SingleResult;
    fn new(
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: &'matrix CsMatI<Pattern, u32>,
        config: &'config ConfigV2,
        mapping: Self::Mapping,
    ) -> Self;
    fn get_evil_row_ids(&self) -> usize;
    fn get_evil_col_ids(&self) -> usize;
    fn get_mapping(&self) -> &Self::Mapping;
    fn evil_row_contains(&self, row_id: usize) -> bool;
    fn evil_col_contains(&self, col_id: usize) -> bool;
    fn handle_evil_row(&mut self, target_id: LogicRowId, mat_b_row_id: LogicRowId) {
        self.handle_distribute_default(
            target_id,
            mat_b_row_id,
            LogicColId(0),
            DistributeType::EvilRow,
        );
    }
    fn handle_evil_col(
        &mut self,
        target_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        mat_b_col_id: LogicColId,
        current_round_vec: &mut Vec<(LogicRowId, LogicColId)>,
    ) {
        self.handle_distribute_default(
            target_id,
            mat_b_row_id,
            mat_b_col_id,
            DistributeType::EvilCol,
        );
    }
    fn handle_distribute_local(
        &mut self,
        target_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        mat_b_col_id: LogicColId,
    ) {
        self.handle_distribute_default(
            target_id,
            mat_b_row_id,
            mat_b_col_id,
            DistributeType::Local,
        );
    }
    fn handle_distribute_remote(
        &mut self,
        target_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        mat_b_col_id: LogicColId,
    ) {
        self.handle_distribute_default(
            target_id,
            mat_b_row_id,
            mat_b_col_id,
            DistributeType::Remote,
        );
    }
    fn handle_reduce_distribute_evil_col(
        &mut self,
        target_id: LogicRowId,
        current_round_vec: &mut Vec<(LogicRowId, LogicColId)>,
    ) {
    }
    fn handle_distribute_default(
        &mut self,
        target_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        mat_b_col_id: LogicColId,
        distribute_type: DistributeType,
    ) {
    }

    fn handle_update_stats(&mut self) {}

    fn get_matrix_b(&self) -> &CsMatI<Pattern, u32>;
    fn run(&mut self, input_vec: &CsMatI<Pattern, u32>, current_batch: usize, _current_topk: f32) {
        let now = std::time::Instant::now();
        debug!("run gearbox sim");

        // distribute the task to components
        let total_rows = input_vec.rows();
        // print every 1% or every 60s
        let mut next_print_percent = total_rows / 100;
        let mut next_print_time = TIME_TO_LOG;
        //each data size if 8 bytes and there are 512 rows in a subarray

        for (target_id, row) in input_vec.outer_iterator().enumerate() {
            if target_id >= next_print_percent || now.elapsed().as_secs() >= next_print_time {
                let time = now.elapsed().as_secs_f32();
                let min = time / 60.;
                let remaining = time / target_id as f32 * (total_rows - target_id) as f32;
                let min_r = remaining / 60.;
                let speed = target_id as f32 / min;
                tracing::trace!("{target_id} of {total_rows} rows processed, time eclips: {min:.2}, estimate remaining time:{min_r:.2},speed: {speed} rows per min");
                next_print_percent = target_id + total_rows / 100;
                next_print_time = now.elapsed().as_secs() + TIME_TO_LOG;
                // if next_print_time > 3000 {
                //     break;
                // }
                if stop_signal::read() {
                    info!("received stop signal, start writing results");
                    break;
                }
            }
            // fix bug here, we should collect the evil col for each target id
            let mut evil_col_row_id_col_id: Vec<(LogicRowId, LogicColId)> = vec![];

            // get the result for that line
            for &mat_b_row_id in row.indices() {
                let mat_b_row_id = mat_b_row_id as usize;
                if self.evil_row_contains(mat_b_row_id) {
                    self.handle_evil_row(LogicRowId(target_id), LogicRowId(mat_b_row_id));
                } else {
                    // the row is not evil, need to access remote
                    let row = self
                        .get_matrix_b()
                        .outer_view(mat_b_row_id)
                        .unwrap()
                        .to_owned();
                    for col in row.indices().iter().map(|i| *i as usize) {
                        if self.evil_col_contains(col) {
                            // the col is evil, no need to access remote
                            // self.hardware
                            //     .distribute_evil_col(target_id, *mat_b_row_id, *col);
                            // evil_col_row_id_col_id
                            //     .push((LogicRowId(mat_b_row_id), LogicColId(col)));
                            self.handle_evil_col(
                                LogicRowId(target_id),
                                LogicRowId(mat_b_row_id),
                                LogicColId(col),
                                &mut evil_col_row_id_col_id,
                            );
                        } else {
                            // the col is not evil, need to access remote
                            let target_partition = self
                                .get_mapping()
                                .get_result_dense_location(target_id.into(), LogicColId(col))
                                .0;
                            let source_partition = self
                                .get_mapping()
                                .get_matrix_b_location(mat_b_row_id.into())
                                .0;
                            if target_partition == source_partition {
                                // self.hardware.distribute_local(
                                //     LogicRowId(target_id),
                                //     LogicRowId(mat_b_row_id),
                                //     LogicColId(col),
                                // );
                                self.handle_distribute_local(
                                    LogicRowId(target_id),
                                    LogicRowId(mat_b_row_id),
                                    LogicColId(col),
                                );
                            } else {
                                // the col is in different partition, need to access remote
                                // self.hardware.read_local_and_distribute_remote(
                                //     LogicRowId(target_id),
                                //     self.hardware
                                //         .mapping
                                //         .get_partition_id_row(LogicRowId(mat_b_row_id)),
                                //     self.hardware
                                //         .mapping
                                //         .get_partition_id_row(LogicRowId(mat_b_row_id)),
                                //     LogicRowId(mat_b_row_id),
                                //     LogicColId(col),
                                // );
                                self.handle_distribute_remote(
                                    LogicRowId(target_id),
                                    LogicRowId(mat_b_row_id),
                                    LogicColId(col),
                                );
                            }
                        }
                    }
                }
            }
            // fix bug here, we should collect the evil col for each target id
            // self.hardware
            //     .distribute_evil_col(LogicRowId(target_id), evil_col_row_id_col_id);
            self.handle_reduce_distribute_evil_col(
                LogicRowId(target_id),
                &mut evil_col_row_id_col_id,
            );
            // reduce the tasks and clear the tasks
            // the cycle of this round
            if (target_id + 1) % current_batch == 0 {
                // update_stats(&mut self.hardware, &mut global_stats);
                self.handle_update_stats();
                // the data for overflow:
            }
            // add the result to the total result and continue to the next line
        }
    }
    fn report(&self, name: String, batch: usize, topk: f32) -> Self::SingleResult;
}
type BatchTopk = (usize, f32);

/// the tool to perform analysis of gearbox of multiple config and graphs
pub trait AnalyzeTool {
    /// the result that reported by the tool
    type ResultType: Send;
    /// the memory size of each subarray that should be allocated
    const SUBARRAY_SIZE: usize;
    /// the simulator type that run a single simulation
    type GearboxSimType<'matrix, 'config, T: Mapping>: GearboxSimTrait<
        'matrix,
        'config,
        Mapping = T,
        SingleResult = Self::ResultType,
    >;
    /// analyze the gearbox of certain dram type
    fn analyze_gearbox_inner<LevelType: LevelTrait>(
        config: &ConfigV2,
        _total_size: &LevelType::Storage,
    ) -> Vec<(BatchTopk, Vec<Self::ResultType>)>
    where
        LevelType::Storage: Debug + Sync,
        LevelType::Mapping: Debug,
    {
        let total_graphs = config.graph_path.len();
        let total_configs = config.gearbox_config.batch.len() * config.gearbox_config.topk.len();
        let total_tasks = total_graphs * total_configs;
        info!(?total_graphs, ?total_configs, ?total_tasks, "total tasks");
        *TOTAL_TASKS.write().unwrap() = total_tasks;

        let results: Vec<_> = config
            .graph_path
            .par_iter()
            .enumerate()
            .map(|(index, path)| {
                info!("analyzing graph {}/{}", index + 1, total_graphs);
                Self::compute_gearbox(config, path)
            })
            .collect();
        let results = super::transpose2(results);
        let configs = config
            .gearbox_config
            .batch
            .iter()
            .cloned()
            .cartesian_product(config.gearbox_config.topk.iter().cloned())
            .collect_vec();
        info!(?configs, "configs");
        assert_eq!(configs.len(), results.len());
        configs.into_iter().zip(results).collect()
    }
    /// the entry point of the analysis
    /// ## Return
    /// a vector of (config, results)
    fn analyze_gearbox(config: &ConfigV2) -> Vec<(BatchTopk, Vec<Self::ResultType>)> {
        match config.dram_type {
            DramType::DDR3 => unimplemented!(),
            DramType::DDR4 => {
                let total_size = ddr4::Storage::new(
                    config.channels.num,
                    config.ranks.num,
                    config.chips.num,
                    config.bank_groups.num,
                    config.banks.num,
                    config.subarrays,
                    config.rows,
                    config.columns,
                );
                Self::analyze_gearbox_inner::<ddr4::Level>(config, &total_size)
            }
            DramType::LPDDR3 => unimplemented!(),
            DramType::LPDDR4 => unimplemented!(),
            DramType::HBM => unimplemented!(),
            DramType::HBM2 => unimplemented!(),
        }
    }
    /// compute a simgle graph
    fn compute_gearbox(config: &ConfigV2, path: &str) -> Vec<Self::ResultType> {
        // for hbm config, they should be 1!
        assert!(config.ranks.num == 1);
        assert!(config.chips.num == 1);
        assert!(config.bank_groups.num == 1);
        let partitions = config.channels.num
            * config.ranks.num
            * config.chips.num
            * config.bank_groups.num
            * config.banks.num
            * config.subarrays;

        info!(?partitions, "compute gearbox");
        info!("reading mtx file: {}", path);
        let read_time = std::time::Instant::now();
        let matrix_head: MatrixHead<Pattern, u32> =
            sprs::io::read_matrix_market_head(path).unwrap();
        let matrix_size =
            matrix_head.ind_ptr_size() + matrix_head.ind_size() + matrix_head.data_size();
        // two csr csc matrix during runtime
        let matrix_size = matrix_size * 2;
        let sim_size = partitions * Self::SUBARRAY_SIZE * 2;
        let sim_size = matrix_size + sim_size;

        let temp_size = matrix_head.tri_size();

        info!(
            "info there will be {} bytes,start acquire the space",
            sim_size + temp_size
        );
        let mut _guard = crate::acquire_memory_sections([sim_size, temp_size]);
        let _guard_temp = _guard.pop().unwrap();
        let _guard_sim = _guard.pop().unwrap();

        let tri_mat: TriMatI<Pattern, u32> = sprs::io::read_matrix_market(path).unwrap();
        info!(
            "finished read the matrix: time:{:.2} secs",
            read_time.elapsed().as_secs_f32()
        );
        let (matrix_a, matrix_b): (CsMatI<Pattern, u32>, CsMatI<Pattern, u32>) =
            (tri_mat.to_csr(), tri_mat.transpose_view().to_csr());

        drop(tri_mat);
        drop(_guard_temp);

        info!(
            "finished transpose the matrix: time:{:.2} secs",
            read_time.elapsed().as_secs_f32()
        );
        let mat_b_rows = matrix_b.rows();
        let mat_b_cols = matrix_b.cols();

        info!(?mat_b_rows, ?mat_b_cols, "matrix b shape");

        // the nnz of matrix b rows
        let mut mat_b_row_ids = (0..mat_b_rows)
            .zip(matrix_b.outer_iterator().map(|row| row.nnz()))
            .collect_vec();
        // the nnz of matrix b cols
        let mut mat_b_col_ids = (0..mat_b_cols)
            .zip(matrix_a.outer_iterator().map(|row| row.nnz()))
            .collect_vec();
        mat_b_row_ids.sort_by_key(|(_index, nnz)| Reverse(*nnz));
        mat_b_col_ids.sort_by_key(|(_index, nnz)| Reverse(*nnz));
        // draw_distribution(
        //     &mat_b_row_ids,
        //     Path::new(&format!("output/{}-{}", file_name, "mat_b_row_ids.png")),
        // );
        // draw_distribution(
        //     &mat_b_col_ids,
        //     Path::new(&format!("output/{}-{}", file_name, "mat_b_col_ids.png")),
        // );

        let batchs = &config.gearbox_config.batch;
        let topks = &config.gearbox_config.topk;
        let configs = batchs.iter().cartesian_product(topks.iter()).collect_vec();
        info!(?configs, "configs");
        let results = configs
            .par_iter()
            .map(|(batch, top_k)| {
                let batch = **batch;
                let top_k = **top_k;
                let top_rows = (mat_b_row_ids.len() as f32 * top_k) as usize;
                let top_rows = if top_rows == 0 { 1 } else { top_rows };
                info!(?top_rows, "top rows");
                let top_cols = (mat_b_col_ids.len() as f32 * top_k) as usize;
                let top_cols = if top_cols == 0 { 1 } else { top_cols };
                info!(?top_cols, "top cols");
                assert!(top_cols > 0);
                let num_partitions = config.channels.num * config.banks.num * config.subarrays;
                debug!(num_partitions, "new gearbox sim");
                let num_rows = matrix_b.rows();
                let num_cols = matrix_b.cols();
                let row_per_partition = (num_rows + num_partitions - 1) / num_partitions;
                let col_per_partition = (num_cols + num_partitions - 1) / num_partitions;

                match config.mapping {
                    crate::pim::configv2::MappingType::SameSubarray => {
                        let mapping = same_subarray::SameSubarrayMapping::new(
                            config,
                            row_per_partition,
                            col_per_partition,
                        );
                        let mut gearbox = Self::GearboxSimType::<'_, '_, SameSubarrayMapping>::new(
                            mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
                            mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
                            &matrix_b,
                            config,
                            mapping,
                        );
                        info!("start running the sim");
                        gearbox.run(&matrix_a, batch, top_k);
                        TOTAL_FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
                        info!(
                            "finished task: {}/{}",
                            TOTAL_FINISHED_TASKS.load(Ordering::Relaxed),
                            *TOTAL_TASKS.read().unwrap()
                        );
                        gearbox.report(path.to_string(), batch, top_k)
                    }
                    crate::pim::configv2::MappingType::SameBank => {
                        let mapping = same_bank::SameBankMapping::new(
                            config.banks.num,
                            config.channels.num,
                            config.subarrays,
                            config.columns,
                            &matrix_b,
                        );
                        let mut gearbox = Self::GearboxSimType::<'_, '_, SameBankMapping>::new(
                            mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
                            mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
                            &matrix_b,
                            config,
                            mapping,
                        );
                        info!("start running the sim");
                        gearbox.run(&matrix_a, batch, top_k);
                        TOTAL_FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
                        info!(
                            "finished task: {}/{}",
                            TOTAL_FINISHED_TASKS.load(Ordering::Relaxed),
                            *TOTAL_TASKS.read().unwrap()
                        );
                        gearbox.report(path.to_string(), batch, top_k)
                    }
                    crate::pim::configv2::MappingType::SameBankWeightedMapping => {
                        let mapping = SameBankWeightedMapping::new(
                            config.banks.num,
                            config.channels.num,
                            config.subarrays,
                            config.columns,
                            &matrix_b,
                        );
                        let mut gearbox =
                            Self::GearboxSimType::<'_, '_, SameBankWeightedMapping>::new(
                                mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
                                mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
                                &matrix_b,
                                config,
                                mapping,
                            );
                        info!("start running the sim");
                        gearbox.run(&matrix_a, batch, top_k);
                        TOTAL_FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
                        info!(
                            "finished task: {}/{}",
                            TOTAL_FINISHED_TASKS.load(Ordering::Relaxed),
                            *TOTAL_TASKS.read().unwrap()
                        );
                        gearbox.report(path.to_string(), batch, top_k)
                    }
                }
            })
            .collect();
        drop(matrix_a);
        drop(matrix_b);
        drop(_guard_sim);
        results
    }
}

/// give an array of data, return each filed's mean, std, max
/// # Example
/// ```
/// use spmspm_pim::analysis::traits::{get_mean_std_max_from_mapper, ReportStats};
/// use std::collections::BTreeMap;
///
/// #[derive(Debug)]
/// struct Data {
///     a: usize,
///     b: usize,
/// }
/// impl ReportStats for Data {
///     fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)> {
///         let mut res = BTreeMap::new();
///         res.insert("a".to_string(), get_mean_std_max_from_mapper(data_vec, |x| x.a));
///         res.insert("b".to_string(), get_mean_std_max_from_mapper(data_vec, |x| x.b));
///         res
///     }
/// }
/// let data = vec![Data { a: 1, b: 2 }, Data { a: 2, b: 3 }];
/// let res = Data::report_stats(&data);
/// assert_eq!(res["a"], (1.5, 0.5, 2));
/// assert_eq!(res["b"], (2.5, 0.5, 3));
/// ```
pub fn get_mean_std_max_from_mapper<T>(
    data: &[T],
    mapper: impl FnMut(&T) -> usize + Clone,
) -> (f64, f64, usize) {
    get_mean_std_max_from_iter(data.iter().map(mapper))
}

/// give an array of data, return each filed's mean, std, max
/// # Example
/// ```
/// use spmspm_pim::analysis::traits::*;
/// use std::collections::BTreeMap;
///
/// #[derive(Debug)]
/// struct Data {
///     a: usize,
///     b: usize,
/// }
/// impl ReportStats for Data {
///     fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)> {
///         let mut res = BTreeMap::new();
///         res.insert(
///             "a".to_string(),
///             get_mean_std_max_from_iter(data_vec.into_iter().map(|x| x.a)),
///         );
///         res.insert(
///             "b".to_string(),
///             get_mean_std_max_from_iter(data_vec.into_iter().map(|x| x.b)),
///         );
///         res
///     }
/// }
/// let data = vec![Data { a: 1, b: 2 }, Data { a: 2, b: 3 }];
/// let res = Data::report_stats(&data);
/// assert_eq!(res["a"], (1.5, 0.5, 2));
/// assert_eq!(res["b"], (2.5, 0.5, 3));
///
///
/// ```
pub fn get_mean_std_max_from_iter(
    data: impl IntoIterator<Item = usize> + Clone,
) -> (f64, f64, usize) {
    let mean = data.clone().into_iter().map(|x| x as f64).mean();
    let std = data
        .clone()
        .into_iter()
        .map(|x| x as f64)
        .population_std_dev();
    let max = data.into_iter().max().unwrap();
    (mean, std, max)
}

#[cfg(test)]
mod tests {
    use crate::analysis::traits::get_mean_std_max_from_iter;

    #[test]
    fn test_trait() {
        use crate::analysis::traits::{get_mean_std_max_from_mapper, ReportStats};
        use std::collections::BTreeMap;

        #[derive(Debug)]
        struct Data {
            a: usize,
            b: usize,
        }
        impl ReportStats for Data {
            fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)> {
                let mut res = BTreeMap::new();
                res.insert(
                    "a".to_string(),
                    get_mean_std_max_from_mapper(data_vec, |x| x.a),
                );
                res.insert(
                    "b".to_string(),
                    get_mean_std_max_from_mapper(data_vec, |x| x.b),
                );
                res
            }
        }
        let data = vec![Data { a: 1, b: 2 }, Data { a: 2, b: 3 }];
        let res = Data::report_stats(&data);
        assert_eq!(res["a"], (1.5, 0.5, 2));
        assert_eq!(res["b"], (2.5, 0.5, 3));
    }
    #[test]
    fn test_trait2() {
        use crate::analysis::traits::ReportStats;
        use std::collections::BTreeMap;

        #[derive(Debug)]
        struct Data {
            a: usize,
            b: usize,
        }
        impl ReportStats for Data {
            fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)> {
                let mut res = BTreeMap::new();
                res.insert(
                    "a".to_string(),
                    get_mean_std_max_from_iter(data_vec.iter().map(|x| x.a)),
                );
                res.insert(
                    "b".to_string(),
                    get_mean_std_max_from_iter(data_vec.iter().map(|x| x.b)),
                );
                res
            }
        }
        let data = vec![Data { a: 1, b: 2 }, Data { a: 2, b: 3 }];
        let res = Data::report_stats(&data);
        assert_eq!(res["a"], (1.5, 0.5, 2));
        assert_eq!(res["b"], (2.5, 0.5, 3));
    }
}
