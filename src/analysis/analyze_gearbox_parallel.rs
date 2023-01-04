//! this module is used to analyze the gearbox
//! # WARNING:
//!
//! !!! this module is derived from analyze_split_spmm.rs and the code and ***doc*** might not be accurate
//!
//! the parallel version of the gearbox analysis
use hashbrown::HashSet;
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    sync::{atomic::AtomicUsize, Mutex},
};

use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
};
use rayon::prelude::IntoParallelRefIterator;
use rayon::{iter::ParallelIterator, prelude::ParallelBridge};
use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat, TriMat};
use tracing::{debug, info};

/// the statistics of a single graph
#[derive(Serialize, Deserialize)]
pub struct SingleResult {
    pub name: String,
    pub subarray_result: Vec<SubArrayResult>,
    pub ring_result: Vec<RingResult>,
    pub tsv_result: Vec<TsvResult>,
}
#[derive(Serialize, Deserialize)]
/// the statistics of all graphs
pub struct GearboxReslt {
    /// the statistics of all graphs
    pub results: Vec<SingleResult>,
}

impl GearboxReslt {
    /// print out all the results
    #[allow(unused)]
    pub fn show_results(&self) {
        unimplemented!()
    }
}

impl Debug for GearboxReslt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for GearboxReslt {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!();
    }
}

/// analyze the split spmm
pub(crate) fn analyze_gearbox(config: &Config) -> GearboxReslt {
    match config.dram_type {
        crate::pim::config::DramType::DDR3 => unimplemented!(),
        crate::pim::config::DramType::DDR4 => {
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
            analyze_gearbox_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => unimplemented!(),
        crate::pim::config::DramType::LPDDR4 => unimplemented!(),
        crate::pim::config::DramType::HBM => unimplemented!(),
        crate::pim::config::DramType::HBM2 => unimplemented!(),
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct GearboxConfig {
    pub topk: f32,
    pub stacks: usize,
    pub layers: usize,
}

struct SubArray {
    inner: Mutex<SubArrayInner>,
}
struct SubArrayInner {
    read_open: Option<usize>,
    write_open: Option<usize>,
    local_read_rows: Vec<(PhysicRowId, usize)>,
    local_write_rows: Vec<(PhysicRowId, usize)>,
}
#[derive(Serialize, Deserialize)]
pub struct SubArrayResult {
    pub cycle: usize,
    pub row_open_cycle: usize,
    pub row_read_cycle: usize,
    pub row_write_cycle: usize,
    pub comp_cycle: usize,
}
impl SubArrayInner {
    /// create a new subarray
    fn new() -> Self {
        Self {
            read_open: None,
            write_open: None,
            local_read_rows: Vec::new(),
            local_write_rows: Vec::new(),
        }
    }
    /// a local read and write task(local accumulate)
    fn add_task(&mut self, local_read: PhysicRowId, local_write: PhysicRowId) {
        match self.local_read_rows.last() {
            Some((last_read, _)) if *last_read == local_read => {
                self.local_read_rows.last_mut().unwrap().1 += 1;
            }
            _ => {
                self.local_read_rows.push((local_read, 1));
            }
        }
        match self.local_write_rows.last() {
            Some((last_write, _)) if *last_write == local_write => {
                self.local_write_rows.last_mut().unwrap().1 += 1;
            }
            _ => {
                self.local_write_rows.push((local_write, 1));
            }
        }
    }

    fn report(&self) -> SubArrayResult {
        let mut row_open_cycle = 0;
        let mut row_read_cycle = 0;
        let mut row_write_cycle = 0;
        let mut comp_cycle = 0;
        let mut cycle = 0;
        for (row_id, read_times) in self.local_read_rows.iter() {
            match self.read_open {
                Some(row) => {
                    if row == row_id.0 {
                    } else {
                        cycle += 18;
                        row_open_cycle += 18;
                        row_read_cycle += 18;
                    }
                }
                None => {
                    cycle += 9;
                    row_open_cycle += 9;
                    row_read_cycle += 9;
                }
            }
            cycle += read_times;
            comp_cycle += read_times;
        }
        for (row_id, read_times) in self.local_write_rows.iter() {
            match self.write_open {
                Some(row) => {
                    if row == row_id.0 {
                    } else {
                        cycle += 18;
                        row_open_cycle += 18;
                        row_write_cycle += 18;
                    }
                }
                None => {
                    cycle += 9;
                    row_open_cycle += 9;
                    row_write_cycle += 9;
                }
            }
            cycle += read_times;
        }
        SubArrayResult {
            cycle,
            row_open_cycle,
            row_read_cycle,
            row_write_cycle,
            comp_cycle,
        }
    }
    /// after received the remote task, it will update the local dense result
    fn add_remote_task(&mut self, local_write: PhysicRowId) {
        match self.local_write_rows.last() {
            Some((last_write, _)) if *last_write == local_write => {
                self.local_write_rows.last_mut().unwrap().1 += 1;
            }
            _ => {
                self.local_write_rows.push((local_write, 1));
            }
        }
    }
}

struct Ring {
    tasks: Mutex<Vec<(RingPort, RingPort)>>,
    ports: u8,
}

#[derive(Serialize, Deserialize)]
pub struct RingResult {
    pub cycle: usize,
    pub traffic: usize,
}

impl Ring {
    fn new(ports: u8) -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            ports,
        }
    }
    fn add_task(&self, source: RingPort, target: RingPort) {
        self.tasks.lock().unwrap().push((source, target));
    }

    fn report(&self) -> RingResult {
        // simulate the ring process
        let mut pathes = vec![0; self.ports as usize];
        for (source, target) in self.tasks.lock().unwrap().iter() {
            let forward_len = (target.0 + self.ports - source.0) % self.ports;
            let backward_len = (source.0 + self.ports - target.0) % self.ports;
            let (from, to) = if forward_len < backward_len {
                (source.0, target.0)
            } else {
                (target.0, source.0)
            };
            for i in from..to {
                pathes[i as usize] += 1;
            }
        }

        RingResult {
            cycle: *pathes.iter().max().unwrap_or(&0),
            traffic: self.tasks.lock().unwrap().len(),
        }
    }
}

struct Tsv {
    traffic: AtomicUsize,
}
#[derive(Serialize, Deserialize)]
pub struct TsvResult {
    pub cycle: usize,
    pub traffic: usize,
}
impl Tsv {
    fn new() -> Self {
        Self {
            traffic: AtomicUsize::new(0),
        }
    }
    fn add_task(&self) {
        self.traffic
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn report(&self) -> TsvResult {
        TsvResult {
            cycle: self.traffic.load(std::sync::atomic::Ordering::Relaxed),
            traffic: self.traffic.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct LogicRowId(usize);

/// the col id in matrix(0..matrix_cols)
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct LogicColId(usize);

/// the row id in a subarray(0..subarray_rows)
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct PhysicRowId(usize);

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct SubarrayId(usize);

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct RingId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct TsvId(usize);

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct RingPort(u8);

struct Hardware {
    sub_array: Vec<SubArray>,
    ring: Vec<Ring>,
    tsv: Vec<Tsv>,
    config: Config,
    /// the dimension of dense matrix in one subarray
    dense_dim: usize,
    /// for normal rows, distrubute them to different subarrays
    row_per_partition: usize,
    /// for target rows, distrubute the cols to different subarrays
    /// and for the evil row, distrubute them by column
    col_per_partition: usize,
}

impl Hardware {
    fn new(
        num_subarray: usize,
        num_rings: usize,
        num_tsvs: usize,
        dense_dim: usize,
        config: Config,
        row_per_partition: usize,
        col_per_partition: usize,
    ) -> Self {
        // each single layer should be a channel
        assert!(
            config.gearbox_config.stacks * config.gearbox_config.layers == config.channels.num,
            "the number of stacks and layers should be equal to the number of channels"
        );
        Self {
            sub_array: (0..num_subarray)
                .map(|_| SubArray {
                    inner: Mutex::new(SubArrayInner::new()),
                })
                .collect(),
            ring: (0..num_rings)
                .map(|_| Ring::new(config.banks.num as u8))
                .collect(),
            tsv: (0..num_tsvs).map(|_| Tsv::new()).collect(),
            dense_dim,
            config,
            row_per_partition,
            col_per_partition,
        }
    }
    #[allow(unused)]
    fn subarrays(&self) -> usize {
        self.sub_array.len()
    }
    #[allow(unused)]
    fn banks(&self) -> usize {
        self.config.channels.num
            * self.config.ranks.num
            * self.config.chips.num
            * self.config.bank_groups.num
            * self.config.banks.num
    }
    /// num of logic layers
    #[allow(unused)]
    fn layers(&self) -> usize {
        self.config.gearbox_config.layers
    }

    /// num of stacks
    #[allow(unused)]
    fn stacks(&self) -> usize {
        self.config.gearbox_config.stacks
    }
    #[allow(unused)]
    fn rings(&self) -> usize {
        self.ring.len()
    }
    #[allow(unused)]
    fn tsvs(&self) -> usize {
        self.tsv.len()
    }

    fn distribute_evil_row(
        &self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_ids: impl IntoIterator<Item = LogicColId>,
    ) {
        // evil row will always be local
        for col_id in col_ids {
            self.distribute_local_evil_row(target_row_id, mat_b_row_id, col_id);
        }
    }
    /// the total evil cols belongs to this target
    fn distribute_evil_col(
        &self,
        target_row_id: LogicRowId,
        row_id_col_id: impl IntoIterator<Item = (LogicRowId, LogicColId)>,
    ) {
        // write to local buffer then write to remote
        // step 1: write to local buffer
        // should be ignored
        // step 2: write to remote once the whole column is finished

        let mut remote_tasks = BTreeMap::new();
        let mut dispatcher_id = None;
        for (mat_b_row_id, col_id) in row_id_col_id {
            let partition_id = self.get_partition_id_row(target_row_id);
            let target_partition_id = self.get_partition_id_col(col_id);
            if partition_id == target_partition_id {
                self.distribute_local(target_row_id, mat_b_row_id, col_id);
            } else {
                // record the remote tasks
                *remote_tasks.entry(col_id).or_insert(0usize) += 1;
                match dispatcher_id {
                    Some(_id) => {}
                    _ => {
                        dispatcher_id = Some(partition_id);
                    }
                }
            }
        }
        // send the remote to ring, tsv and remote subarray
        for entry in remote_tasks {
            let col_id = entry.0;
            // let count = entry.1;
            self.distribute_remote(target_row_id, dispatcher_id.unwrap(), col_id);
        }
    }

    fn distribute_local_evil_row(
        &self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.get_partition_id_col(col_id);
        let local_read = self.get_row_id_evil(mat_b_row_id, col_id);
        let local_write = self.get_row_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0]
            .inner
            .lock()
            .unwrap()
            .add_task(local_read, local_write);
    }

    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }
    fn get_dispatcher_id(&self, sub_array_id: SubarrayId) -> SubarrayId {
        // the dispatcher is the first subarray of the same bank
        SubarrayId(sub_array_id.0 - sub_array_id.0 % self.config.subarrays)
    }
    fn distribute_local(
        &self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.get_partition_id_row(mat_b_row_id);
        let local_read = self.get_row_id(mat_b_row_id, col_id);
        let local_write = self.get_row_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0]
            .inner
            .lock()
            .unwrap()
            .add_task(local_read, local_write);
    }

    fn get_tsv_id_from_subarray(&self, sub_array_id: SubarrayId) -> TsvId {
        TsvId(sub_array_id.0 / self.config.subarrays / self.config.banks.num)
    }
    #[allow(unused)]
    fn get_tsv_id_from_ring(&self, ring_id: RingId) -> TsvId {
        // the ring id is the same as the tsv id
        TsvId(ring_id.0)
    }

    fn ring_port_from_subarray(&self, subarray_id: SubarrayId) -> RingPort {
        RingPort(((subarray_id.0 / self.config.subarrays) % self.config.banks.num) as u8)
    }
    fn distribute_remote(
        &self,
        target_row_id: LogicRowId,
        dispatcher_id: SubarrayId,
        col_id: LogicColId,
    ) {
        // first write to the rings
        let source_layer = self.ring_id_from_subarray(dispatcher_id);
        let target_partition_id = self.get_partition_id_col(col_id);
        let target_layer = self.ring_id_from_subarray(target_partition_id);
        if source_layer == target_layer {
            // no need to write to the csv
            let source = self.ring_port_from_subarray(dispatcher_id);

            let target = self.ring_port_from_subarray(target_partition_id);
            self.ring[source_layer.0].add_task(source, target);
        } else {
            // write to source ring
            let source_bank = self.ring_port_from_subarray(dispatcher_id);
            let target_bank = RingPort(0);
            self.ring[source_layer.0].add_task(source_bank, target_bank);

            // write to tsv from source ring to base ring
            let tsv_id = self.get_tsv_id_from_subarray(dispatcher_id);
            self.tsv[tsv_id.0].add_task();

            // write to the base icnt
            // ignored because it's not the bottleneck

            // write to tsv from base ring to target ring
            let tsv_id = self.get_tsv_id_from_subarray(target_partition_id);
            self.tsv[tsv_id.0].add_task();

            // write to target ring
            let source_bank = RingPort(0);
            let target_bank = self.ring_port_from_subarray(target_partition_id);
            self.ring[target_layer.0].add_task(source_bank, target_bank);

            // target subarray distribute local task
            let remote_dense_row_write = self.get_row_id_dense(target_row_id, col_id);
            self.sub_array[target_partition_id.0]
                .inner
                .lock()
                .unwrap()
                .add_remote_task(remote_dense_row_write);
        }
    }
    /// from bank id to ring id
    fn ring_id_from_subarray(&self, partition_id: SubarrayId) -> RingId {
        let bank_id = partition_id.0 / self.config.subarrays;
        RingId(bank_id / self.config.banks.num)
    }

    fn get_row_id(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }

    fn get_row_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        PhysicRowId((target_row_id.0 * self.dense_dim + col_id.0) / 256)
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        // the rows are distrubuted to every subarray
        SubarrayId(row_id.0 / self.row_per_partition)
    }

    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        // the cols are distrubuted to every subarray
        SubarrayId(col_id.0 / self.col_per_partition)
    }
    /// reduce the result and return the result
    fn report(&self, name: String) -> SingleResult {
        // reduce the result
        let subarray_result: Vec<_> = self
            .sub_array
            .par_iter()
            .map(|sub_array| sub_array.inner.lock().unwrap().report())
            .collect();
        let ring_result: Vec<_> = self.ring.par_iter().map(|ring| ring.report()).collect();
        let tsv_result: Vec<_> = self.tsv.par_iter().map(|tsv| tsv.report()).collect();
        SingleResult {
            name,
            subarray_result,
            ring_result,
            tsv_result,
        }
    }
}

struct GearboxSim {
    row_per_partition: usize,
    #[allow(unused)]
    col_per_partition: usize,
    /// the id of evil col: this col will  have a backup copy in each partition
    evil_col_ids: HashSet<usize>,
    /// the id of evil row: the evil row will be partitioned into each components,there are no remote access needed.
    evil_row_ids: HashSet<usize>,
    matrix_b: CsMat<Pattern>,
    hardware: Hardware,
}
impl GearboxSim {
    fn new(
        num_partitions: usize,
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: CsMat<Pattern>,
        config: &Config,
    ) -> Self {
        debug!(num_partitions, "new gearbox sim");
        let num_rows = matrix_b.rows();
        let num_cols = matrix_b.cols();
        let mut row_per_partition = (num_rows + num_partitions - 1) / num_partitions;
        let mut col_per_partition = (num_cols + num_partitions - 1) / num_partitions;
        if row_per_partition == 0 {
            row_per_partition = 1;
        }
        if col_per_partition == 0 {
            col_per_partition = 1;
        }
        assert!(row_per_partition > 0);
        assert!(col_per_partition > 0);
        let num_rings = config.gearbox_config.stacks * config.gearbox_config.layers;
        let mut dense_dim = matrix_b.cols() / num_partitions;
        if dense_dim == 0 {
            dense_dim = 1;
        }
        GearboxSim {
            row_per_partition,
            col_per_partition,
            evil_col_ids: evil_col_ids.into_iter().collect(),
            evil_row_ids: evil_row_ids.into_iter().collect(),
            matrix_b,
            hardware: Hardware::new(
                num_partitions,
                num_rings,
                num_rings,
                dense_dim,
                config.clone(),
                row_per_partition,
                col_per_partition,
            ),
        }
    }

    /// distribute the task to components
    fn run(&mut self, input_mat: &CsMat<Pattern>) {
        debug!("run gearbox sim");
        let evil_rows = self.evil_row_ids.len();
        let evil_cols = self.evil_col_ids.len();
        debug!(?self.row_per_partition,?self.row_per_partition,  ?evil_rows, ?evil_cols, "run gearbox sim");
        debug!(?self.evil_row_ids, ?self.evil_col_ids, "run gearbox sim");
        // distribute the task to components
        input_mat
            .outer_iterator()
            .enumerate()
            .par_bridge()
            .for_each(|(target_id, row)| {
                for mat_b_row_id in row.indices() {
                    if self.evil_row_ids.contains(mat_b_row_id) {
                        // the row is evil, no need to access remote
                        self.hardware.distribute_evil_row(
                            LogicRowId(target_id),
                            LogicRowId(*mat_b_row_id),
                            self.matrix_b
                                .outer_view(*mat_b_row_id)
                                .unwrap()
                                .indices()
                                .iter()
                                .map(|&x| LogicColId(x)),
                        );
                    } else {
                        let mut evil_col_row_id_col_id = vec![];
                        // the row is not evil, need to access remote
                        for col in self.matrix_b.outer_view(*mat_b_row_id).unwrap().indices() {
                            if self.evil_col_ids.contains(col) {
                                // the col is evil, no need to access remote
                                // self.hardware
                                //     .distribute_evil_col(target_id, mat_b_row_id, *col);
                                evil_col_row_id_col_id
                                    .push((LogicRowId(*mat_b_row_id), LogicColId(*col)));
                            } else {
                                // the col is not evil, need to access remote
                                let target_partition = self.get_partition_id_col(LogicColId(*col));
                                let source_partition =
                                    self.get_partition_id_row(LogicRowId(*mat_b_row_id));
                                if target_partition == source_partition {
                                    self.hardware.distribute_local(
                                        LogicRowId(target_id),
                                        LogicRowId(*mat_b_row_id),
                                        LogicColId(*col),
                                    );
                                } else {
                                    // the col is in different partition, need to access remote
                                    self.hardware.distribute_remote(
                                        LogicRowId(target_id),
                                        self.hardware.get_dispatcher_id(
                                            self.hardware
                                                .get_partition_id_row(LogicRowId(*mat_b_row_id)),
                                        ),
                                        LogicColId(*col),
                                    );
                                }
                            }
                        }
                        self.hardware
                            .distribute_evil_col(LogicRowId(target_id), evil_col_row_id_col_id);
                    }
                }
            })
    }

    /// reduce the result and return the result
    fn report(&self, name: String) -> SingleResult {
        self.hardware.report(name)
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        self.hardware.get_partition_id_row(row_id)
    }
    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        self.hardware.get_partition_id_col(col_id)
    }
}
fn compute_gearbox(config: &Config, path: &str) -> SingleResult {
    let partitions = config.channels.num
        * config.ranks.num
        * config.chips.num
        * config.bank_groups.num
        * config.banks.num
        * config.subarrays;
    let matrix_a: TriMat<Pattern> = sprs::io::read_matrix_market(path).unwrap();
    let (matrix_a, matrix_b): (CsMat<Pattern>, CsMat<Pattern>) =
        (matrix_a.to_csr(), matrix_a.transpose_view().to_csr());
    let mat_b_rows = matrix_b.rows();
    let mat_b_cols = matrix_b.cols();

    info!(?mat_b_rows, ?mat_b_cols, "matrix b shape");

    let mut mat_b_row_ids = (0..mat_b_rows)
        .zip(matrix_b.outer_iterator().map(|row| row.nnz()))
        .collect_vec();
    let mut mat_b_col_ids = (0..mat_b_cols)
        .zip(matrix_a.outer_iterator().map(|row| row.nnz()))
        .collect_vec();
    mat_b_row_ids.sort_by_key(|(_index, nnz)| *nnz);
    mat_b_col_ids.sort_by_key(|(_index, nnz)| *nnz);
    let top_rows = (mat_b_row_ids.len() as f32 * config.gearbox_config.topk) as usize;
    let top_rows = if top_rows == 0 { 1 } else { top_rows };

    let top_cols = (mat_b_col_ids.len() as f32 * config.gearbox_config.topk) as usize;
    let top_cols = if top_cols == 0 { 1 } else { top_cols };
    assert!(top_cols > 0);
    let mut gearbox = GearboxSim::new(
        partitions,
        mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
        mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
        matrix_b,
        config,
    );

    gearbox.run(&matrix_a);
    gearbox.report(path.to_string())
}
fn analyze_gearbox_inner<LevelType: LevelTrait>(
    config: &Config,
    _total_size: &LevelType::Storage,
) -> GearboxReslt
where
    LevelType::Storage: Debug + Sync,
    LevelType::Mapping: Debug,
{
    let total_graphs = config.graph_path.len();
    let results = config
        .graph_path
        .iter()
        .enumerate()
        .map(|(index, path)| {
            info!("analyzing graph {}/{}", index + 1, total_graphs);
            compute_gearbox(config, path)
        })
        .collect_vec();

    GearboxReslt { results }
}

/// the stat result of the seq spmm
#[derive(Debug, Serialize)]
pub struct SeqResult {
    /// the cycles
    pub cycle: u64,
    /// the graph name
    pub name: String,
    /// compute cycles
    pub compute_cycle: u64,
    /// the time spent on row_open:
    pub row_open: u64,
    /// the time spent on temp_result_read
    pub temp_result_read: u64,
    /// the time spent on final_result_write
    pub final_result_write: u64,
    /// the time spent on matrix_b_read
    pub matrix_b_read: u64,
    /// the number of bytes that row opens
    pub row_open_bytes: usize,
    /// the number of bytes that really used
    pub used_bytes: usize,
    /// total input read bytes
    pub input_read_bytes: usize,
    /// total input read times
    pub input_read_times: usize,
}
#[allow(unused)]
#[derive(Default, Debug)]
struct SubarrayStatus {
    opened_row: Option<usize>,
    last_read_col: usize,
}

impl SubarrayStatus {
    /// create a new subarray status
    #[allow(unused)]
    fn new() -> Self {
        Default::default()
    }
    /// open a row
    ///
    /// # Arguments
    /// - `start`: the tuple of (row, col)
    /// - `length`: the tuple of (row, col)
    /// - `activate_cycle`: the activate cycle
    /// - `precharge_cycle`: the precharge cycle
    /// - `columns`: the cols of a row
    /// # Return
    /// the tuple (first_row_cycle, remaining_cycle,row_activated)
    #[allow(unused)]
    fn open_row(
        &mut self,
        start: (usize, usize),
        length: usize,
        activate_cycle: usize,
        precharge_cycle: usize,
        columns: usize,
    ) -> (usize, usize, usize) {
        if length == 0 {
            return (0, 0, 0);
        }
        let mut total_rows_activated = 0;
        let first_row_cycle = match self.opened_row {
            Some(row) => {
                if row == start.0 {
                    0
                } else {
                    total_rows_activated += 1;
                    activate_cycle + precharge_cycle
                }
            }
            None => {
                total_rows_activated += 1;
                activate_cycle
            }
        };
        // all remaining rows should be precharged and activated
        let final_col = start.1 + length;
        let remaining_rows = (final_col - 1) / columns;
        let final_row = start.0 + remaining_rows;
        let remaining_cycle = remaining_rows * (activate_cycle + precharge_cycle);
        self.opened_row = Some(final_row);
        self.last_read_col = (final_col - 1) % columns;
        (
            first_row_cycle,
            remaining_cycle,
            total_rows_activated + remaining_rows,
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        init_logger_debug,
        pim::config::{Config, LevelConfig},
    };

    use super::*;

    #[test]
    fn test_split_spmm() {
        init_logger_debug();
        let config = Config {
            channels: LevelConfig {
                num: 1,
                ..Default::default()
            },
            ranks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            chips: LevelConfig {
                num: 1,
                ..Default::default()
            },
            bank_groups: LevelConfig {
                num: 1,
                ..Default::default()
            },
            banks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            graph_path: vec!["mtx/test.mtx".to_string()],
            ..Config::from_ddr4_3200(
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
            )
        };
        let result = analyze_gearbox(&config);
        result.show_results();
    }

    #[test]
    fn test_split_spmm_long_vec() {
        init_logger_debug();
        let config = Config {
            channels: LevelConfig {
                num: 1,
                ..Default::default()
            },
            ranks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            chips: LevelConfig {
                num: 1,
                ..Default::default()
            },
            bank_groups: LevelConfig {
                num: 1,
                ..Default::default()
            },
            banks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            graph_path: vec!["mtx/test.mtx".to_string()],
            columns: 8,

            ..Config::from_ddr4_3200(
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
            )
        };

        let result = analyze_gearbox(&config);
        result.show_results();
    }

    #[test]
    fn test_open_row() {
        init_logger_debug();
        let mut subarray = SubarrayStatus::default();
        let result = subarray.open_row((0, 13), 100, 10, 30, 20);
        assert_eq!((10, 200, 0), result);
        assert_eq!(Some(5), subarray.opened_row);
        assert_eq!(12, subarray.last_read_col);
    }
}
