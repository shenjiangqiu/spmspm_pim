//! this module is used to analyze the gearbox
//! # WARNING:
//!
//! !!! this module is derived from analyze_split_spmm.rs and the code and ***doc*** might not be accurate
use std::cmp::Reverse;
use std::error::Error;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
};

use hashbrown::HashSet;
use itertools::Itertools;
use plotters::coord::Shift;
use plotters::prelude::*;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat, TriMat};
use tracing::{debug, info};

use crate::draw::DrawFn;
use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
};

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
pub struct GearboxResult {
    /// the statistics of all graphs
    pub results: Vec<SingleResult>,
}

impl GearboxResult {
    /// print out all the results
    #[allow(unused)]
    pub fn show_results(&self) {
        unimplemented!()
    }
}

impl Debug for GearboxResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for GearboxResult {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!();
    }
}

/// analyze the split spmm
pub(crate) fn analyze_gearbox(config: &Config) -> GearboxResult {
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

#[derive(Clone)]
struct SubArray {
    read_open: Option<usize>,
    write_open: Option<usize>,

    remote_write: Option<usize>,

    sub_array_result: SubArrayResult,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SubArrayResult {
    pub cycle: usize,
    // for normal rows
    pub local_row_open_cycle: usize,
    pub local_row_read_cycle: usize,
    pub local_row_write_cycle: usize,

    pub comp_cycle: usize,

    // for evil row
    pub local_row_open_cycle_evil: usize,
    pub local_row_read_cycle_evil: usize,
    pub local_row_write_cycle_evil: usize,
    // for remote rows that read by local subarray
    pub remote_row_read_cycle: usize,

    // remote result write by target subarray
    pub remote_row_write_cycle: usize,
    pub cycle_remote: usize,
}

impl SubArray {
    /// create a new subarray
    fn new() -> Self {
        Self {
            read_open: None,
            write_open: None,
            remote_write: None,
            sub_array_result: Default::default(),
        }
    }

    /// a local read and write task(local accumulate)
    fn add_task(&mut self, local_read: PhysicRowId, local_write: PhysicRowId, evil: bool) {
        if evil {
            match self.read_open {
                Some(last_read) => {
                    if last_read == local_read.0 {
                        self.sub_array_result.local_row_read_cycle_evil += 1;
                        self.sub_array_result.cycle += 1;
                    } else {
                        self.sub_array_result.local_row_open_cycle_evil += 19;
                        self.sub_array_result.cycle += 19;
                        self.read_open = Some(local_read.0);
                    }
                }
                _ => {
                    self.sub_array_result.local_row_open_cycle_evil += 9;
                    self.sub_array_result.cycle += 9;
                    self.read_open = Some(local_read.0);
                }
            }
            match self.write_open {
                Some(last_write) => {
                    if last_write == local_write.0 {
                        self.sub_array_result.local_row_write_cycle_evil += 1;
                    } else {
                        self.sub_array_result.local_row_open_cycle_evil += 19;
                        self.sub_array_result.cycle += 19;
                        self.write_open = Some(local_write.0);
                    }
                }
                _ => {
                    self.sub_array_result.local_row_open_cycle_evil += 9;
                    self.sub_array_result.cycle += 9;
                    self.write_open = Some(local_write.0);
                }
            }
        } else {
            // not evil
            match self.read_open {
                Some(last_read) => {
                    if last_read == local_read.0 {
                        self.sub_array_result.local_row_read_cycle += 1;
                        self.sub_array_result.cycle += 1;
                    } else {
                        self.sub_array_result.local_row_open_cycle += 19;
                        self.sub_array_result.cycle += 19;
                        self.read_open = Some(local_read.0);
                    }
                }
                _ => {
                    self.sub_array_result.local_row_open_cycle += 9;
                    self.sub_array_result.cycle += 9;
                    self.read_open = Some(local_read.0);
                }
            }
            match self.write_open {
                Some(last_write) => {
                    if last_write == local_write.0 {
                        self.sub_array_result.local_row_write_cycle += 1;
                    } else {
                        self.sub_array_result.local_row_open_cycle += 19;
                        self.sub_array_result.cycle += 19;
                        self.write_open = Some(local_write.0);
                    }
                }
                _ => {
                    self.sub_array_result.local_row_open_cycle += 9;
                    self.sub_array_result.cycle += 9;
                    self.write_open = Some(local_write.0);
                }
            }
        }
    }

    /// the tasks that needed to send to the remote should be read local row first.
    fn add_remote_read_task(&mut self, local_read: PhysicRowId) {
        match self.read_open {
            Some(last_read) => {
                if last_read == local_read.0 {
                    self.sub_array_result.remote_row_read_cycle += 1;
                    self.sub_array_result.cycle += 1;
                } else {
                    self.sub_array_result.remote_row_read_cycle += 19;
                    self.sub_array_result.cycle += 19;
                    self.read_open = Some(local_read.0);
                }
            }
            _ => {
                self.sub_array_result.remote_row_read_cycle += 9;
                self.sub_array_result.cycle += 9;
                self.read_open = Some(local_read.0);
            }
        }
    }

    /// after received the remote task, it will update the local dense result
    fn add_remote_task(&mut self, local_write: PhysicRowId) {
        match self.remote_write {
            Some(last_write) => {
                if last_write == local_write.0 {
                    self.sub_array_result.remote_row_write_cycle += 1;
                } else {
                    self.sub_array_result.remote_row_write_cycle += 19;
                    self.sub_array_result.cycle_remote += 19;
                    self.remote_write = Some(local_write.0);
                }
            }
            _ => {
                self.sub_array_result.remote_row_write_cycle += 9;
                self.sub_array_result.cycle_remote += 9;
                self.remote_write = Some(local_write.0);
            }
        }
    }

    fn report(&self) -> SubArrayResult {
        self.sub_array_result.clone()
    }
}

#[derive(Clone)]
struct Ring {
    tasks: Vec<(RingPort, RingPort)>,
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
            tasks: Vec::new(),
            ports,
        }
    }
    fn add_task(&mut self, source: RingPort, target: RingPort) {
        self.tasks.push((source, target));
    }

    fn report(&self) -> RingResult {
        // simulate the ring process
        let mut paths = vec![0; self.ports as usize];
        for (source, target) in self.tasks.iter() {
            let forward_len = (target.0 + self.ports - source.0) % self.ports;
            let backward_len = (source.0 + self.ports - target.0) % self.ports;
            let (from, to) = if forward_len < backward_len {
                (source.0, target.0)
            } else {
                (target.0, source.0)
            };
            for i in from..to {
                paths[i as usize] += 1;
            }
        }

        RingResult {
            cycle: *paths.iter().max().unwrap_or(&0),
            traffic: self.tasks.len(),
        }
    }
}

#[derive(Clone)]
struct Tsv {
    traffic: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TsvResult {
    pub cycle: usize,
    pub traffic: usize,
}

impl Tsv {
    fn new() -> Self {
        Self { traffic: 0 }
    }
    fn add_task(&mut self) {
        self.traffic += 1;
    }

    fn report(&self) -> TsvResult {
        TsvResult {
            cycle: self.traffic,
            traffic: self.traffic,
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

pub struct Hardware {
    sub_array: Vec<SubArray>,
    ring: Vec<Ring>,
    tsv: Vec<Tsv>,
    config: Config,
    /// the dimension of dense matrix in one subarray
    dense_dim: usize,
    /// for normal rows, distribute them to different subarrays
    row_per_partition: usize,
    /// for target rows, distribute the cols to different subarrays
    /// and for the evil row, distribute them by column
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
        assert_eq!(
            config.gearbox_config.stacks * config.gearbox_config.layers,
            config.channels.num,
            "the number of stacks and layers should be equal to the number of channels"
        );
        Self {
            sub_array: vec![SubArray::new(); num_subarray],
            ring: vec![Ring::new(config.banks.num as u8); num_rings],
            tsv: vec![Tsv::new(); num_tsvs],
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
        &mut self,
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
        &mut self,
        target_row_id: LogicRowId,
        row_id_col_id: impl IntoIterator<Item = (LogicRowId, LogicColId)>,
    ) {
        // write to local buffer then write to remote

        // fix bug here,

        // need to read local first

        // step 1: write to local buffer
        // should be ignored
        // step 2: write to remote once the whole column is finished

        let mut remote_tasks = BTreeMap::new();
        let mut local_subarray_id = None;
        for (mat_b_row_id, col_id) in row_id_col_id {
            let partition_id = self.get_partition_id_row(target_row_id);
            let target_partition_id = self.get_partition_id_col(col_id);
            if partition_id == target_partition_id {
                self.distribute_local(target_row_id, mat_b_row_id, col_id);
            } else {
                // record the remote tasks
                // first read the local row
                let physic_row_id = self.get_row_id(mat_b_row_id, col_id);
                self.sub_array[partition_id.0].add_remote_read_task(physic_row_id);
                // then store the temporary result for the remote dense
                *remote_tasks.entry(col_id).or_insert(0usize) += 1;
                match local_subarray_id {
                    Some(_id) => {}
                    _ => {
                        local_subarray_id = Some(partition_id);
                    }
                }
            }
        }
        let dispatcher_id =
            local_subarray_id.map(|subarray_id| self.get_dispatcher_id(subarray_id));
        // send the remote to ring, tsv and remote subarray
        for entry in remote_tasks {
            let col_id = entry.0;
            // let count = entry.1;
            self.distribute_remote(target_row_id, dispatcher_id.unwrap(), col_id);
        }
    }

    fn distribute_local_evil_row(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.get_partition_id_col(col_id);
        let local_read = self.get_row_id_evil(mat_b_row_id, col_id);
        let local_write = self.get_row_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0].add_task(local_read, local_write, true);
    }

    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }
    fn get_dispatcher_id(&self, sub_array_id: SubarrayId) -> SubarrayId {
        // the dispatcher is the first subarray of the same bank
        SubarrayId(sub_array_id.0 - sub_array_id.0 % self.config.subarrays)
    }
    fn distribute_local(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.get_partition_id_row(mat_b_row_id);
        let local_read = self.get_row_id(mat_b_row_id, col_id);
        let local_write = self.get_row_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0].add_task(local_read, local_write, false);
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
    fn read_local_and_distribute_remote(
        &mut self,
        target_row_id: LogicRowId,
        dispatcher_id: SubarrayId,
        local_subarray: SubarrayId,
        matrix_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // read the local row
        let local_read_row_id = self.get_row_id(matrix_b_row_id, col_id);
        self.sub_array[local_subarray.0].add_remote_read_task(local_read_row_id);
        self.distribute_remote(target_row_id, dispatcher_id, col_id);
    }

    /// this task will have to write the result to a remote subarray
    fn distribute_remote(
        &mut self,
        target_row_id: LogicRowId,
        dispatcher_id: SubarrayId,
        col_id: LogicColId,
    ) {
        //  write to the rings
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
            self.sub_array[target_partition_id.0].add_remote_task(remote_dense_row_write);
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

    /// fix a bug here, the one subarray do not contains the whole dense vec, so the col id should % self.col_per_partition
    fn get_row_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        PhysicRowId((target_row_id.0 * self.dense_dim + col_id.0 % self.col_per_partition) / 256)
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
            .map(|sub_array| sub_array.report())
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
pub struct GearboxSim {
    pub row_per_partition: usize,
    #[allow(unused)]
    pub col_per_partition: usize,
    /// the id of evil col: this col will  have a backup copy in each partition
    pub evil_col_ids: HashSet<usize>,
    /// the id of evil row: the evil row will be partitioned into each components,there are no remote access needed.
    pub evil_row_ids: HashSet<usize>,
    pub matrix_b: CsMat<Pattern>,
    pub hardware: Hardware,
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
    fn run(&mut self, input_vec: &CsMat<Pattern>) {
        debug!("run gearbox sim");
        let evil_rows = self.evil_row_ids.len();
        let evil_cols = self.evil_col_ids.len();
        debug!(?self.row_per_partition,?self.row_per_partition,  ?evil_rows, ?evil_cols, "run gearbox sim");
        debug!(?self.evil_row_ids, ?self.evil_col_ids, "run gearbox sim");
        // distribute the task to components
        for (target_id, row) in input_vec.outer_iterator().enumerate() {
            // get the result for that line
            for &mat_b_row_id in row.indices() {
                if self.evil_row_ids.contains(&mat_b_row_id) {
                    // the row is evil, no need to access remote
                    self.hardware.distribute_evil_row(
                        LogicRowId(target_id),
                        LogicRowId(mat_b_row_id),
                        self.matrix_b
                            .outer_view(mat_b_row_id)
                            .unwrap()
                            .indices()
                            .iter()
                            .map(|&x| LogicColId(x)),
                    );
                } else {
                    let mut evil_col_row_id_col_id = vec![];
                    // the row is not evil, need to access remote
                    for col in self.matrix_b.outer_view(mat_b_row_id).unwrap().indices() {
                        if self.evil_col_ids.contains(col) {
                            // the col is evil, no need to access remote
                            // self.hardware
                            //     .distribute_evil_col(target_id, *mat_b_row_id, *col);
                            evil_col_row_id_col_id
                                .push((LogicRowId(mat_b_row_id), LogicColId(*col)));
                        } else {
                            // the col is not evil, need to access remote
                            let target_partition = self.get_partition_id_col(LogicColId(*col));
                            let source_partition =
                                self.get_partition_id_row(LogicRowId(mat_b_row_id));
                            if target_partition == source_partition {
                                self.hardware.distribute_local(
                                    LogicRowId(target_id),
                                    LogicRowId(mat_b_row_id),
                                    LogicColId(*col),
                                );
                            } else {
                                // the col is in different partition, need to access remote
                                self.hardware.read_local_and_distribute_remote(
                                    LogicRowId(target_id),
                                    self.hardware.get_dispatcher_id(
                                        self.hardware
                                            .get_partition_id_row(LogicRowId(mat_b_row_id)),
                                    ),
                                    self.hardware.get_partition_id_row(LogicRowId(mat_b_row_id)),
                                    LogicRowId(mat_b_row_id),
                                    LogicColId(*col),
                                );
                            }
                        }
                    }
                    self.hardware
                        .distribute_evil_col(LogicRowId(target_id), evil_col_row_id_col_id);
                }
            }
            // add the result to the total result and continue to the next line
        }
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

struct DistributionDrawer;

impl DrawFn for DistributionDrawer {
    type DATA = [(usize, usize)];

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let size = data.len();
        let max_nnz = data.iter().map(|(_, nnz)| *nnz).max().unwrap();

        let mut chart = ChartBuilder::on(&root)
            .set_all_label_area_size(10.percent())
            .build_cartesian_2d(-100..(size + 100) as i32, 0..max_nnz)?;
        chart
            .configure_mesh()
            .disable_x_mesh()
            .disable_y_mesh()
            .x_desc("rows")
            .y_desc("nnzs")
            .axis_desc_style(("sans-serif", 15).into_font())
            .draw()?;
        chart.draw_series(data.iter().enumerate().map(|(id, (_row_id, nnz))| {
            Rectangle::new(
                [(id as i32, 0), (id as i32 + 1, *nnz)],
                RED.mix(0.3).filled(),
            )
        }))?;
        Ok(())
    }
}

fn compute_gearbox(config: &Config, path: &str) -> SingleResult {
    let partitions = config.channels.num
        * config.ranks.num
        * config.chips.num
        * config.bank_groups.num
        * config.banks.num
        * config.subarrays;
    info!(?partitions, "compute gearbox");
    info!("reading mtx file: {}", path);
    let matrix_a: TriMat<Pattern> = sprs::io::read_matrix_market(path).unwrap();
    let (matrix_a, matrix_b): (CsMat<Pattern>, CsMat<Pattern>) =
        (matrix_a.to_csr(), matrix_a.transpose_view().to_csr());
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

    let top_rows = (mat_b_row_ids.len() as f32 * config.gearbox_config.topk) as usize;
    let top_rows = if top_rows == 0 { 1 } else { top_rows };
    info!(?top_rows, "top rows");
    let top_cols = (mat_b_col_ids.len() as f32 * config.gearbox_config.topk) as usize;
    let top_cols = if top_cols == 0 { 1 } else { top_cols };
    info!(?top_cols, "top cols");
    assert!(top_cols > 0);
    info!(
        "the top 10 rows: {:?}",
        mat_b_row_ids.iter().take(10).collect_vec()
    );
    info!(
        "the top 10 cols: {:?}",
        mat_b_col_ids.iter().take(10).collect_vec()
    );

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
) -> GearboxResult
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

    GearboxResult { results }
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

    #[test]
    fn test_gearbox_mapping() {
        // get the number of remote rows, local rows, evil rows, evil cols
    }
}
