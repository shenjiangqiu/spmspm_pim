//! this module is used to analyze the gearbox
//! - date: 2023-03-02
//! # WARNING:
//!
//! !!! this module is derived from analyze_split_spmm.rs and the code and ***doc*** might not be accurate
//!
//!
//! -  
//!
//! ```text
//! ________________________________________
//! / In this version, we get the            \
//! | distribution of the remote traffic and |
//! \ analyze the unbanlance                 /
//!  ----------------------------------------
//!         \   ^__^
//!          \  (oo)\_______
//!             (__)\       )\/\
//!                ||----w |
//!                ||     ||
//! ```
use crate::analysis::mapping::*;
use crate::tools::stop_signal;
use crate::{
    analysis::mapping::Mapping,
    tools::{
        crossbare_simulator::CrossBareSimulator,
        crossbare_simulator_no_conflic::CrossBareSimulatorNoConflict,
        ring_simulator::RingSimulator, CrossBarPacket, Direction, FlatInterleaveTrait, IcntPacket,
    },
};
use hashbrown::HashSet;
use itertools::Itertools;
use plotters::{coord::Shift, prelude::*};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sprs::{io::MatrixHead, num_kinds::Pattern, CsMatI, TriMatI};
use statrs::statistics::*;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt::{Debug, Display},
    mem::size_of,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};
use tracing::{debug, info};

use crate::{
    draw::DrawFn,
    pim::{
        configv2::{ConfigV2, DramType},
        level::{ddr4, LevelTrait},
    },
    TIME_TO_LOG,
};

// #[derive(Serialize, Deserialize, Default, Debug)]
// pub struct TotalResult {
//     pub global_max_acc_cycle: usize,
//     pub global_max_acc_cycle_remote: usize,
//     pub gloabl_max_acc_ring: usize,
//     pub global_max_acc_tsv: usize,
//     pub global_max_real_local: usize,
//     pub global_max_ring_buffer: usize,
//     pub overflow_count_12_256: usize,
//     pub overflow_count_12_512: usize,
//     pub overflow_count_8_256: usize,
//     pub overflow_count_8_512: usize,
//     pub overflow_count_12_256_overhead: usize,
//     pub overflow_count_12_512_overhead: usize,
//     pub overflow_count_8_256_overhead: usize,
//     pub overflow_count_8_512_overhead: usize,
//     pub global_tsv_base_total: usize,
//     pub global_tsv_base_real: usize,
//     pub global_tsv_base_cycle: usize,
// }

/// the statistics of a single graph
#[derive(Serialize, Deserialize, Debug)]
pub struct SingleResult {
    pub name: String,
    pub batch: usize,
    pub topk: f32,
    pub subarray_result: Vec<SubArrayResult>,
    pub ring_result: Vec<RingResult>,
    pub tsv_result: Vec<TsvResult>,
    pub total_result: GlobalStatV2,
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
pub(crate) fn analyze_gearbox(config: &ConfigV2) -> Vec<((usize, f32), Vec<SingleResult>)> {
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
            analyze_gearbox_inner::<ddr4::Level>(config, &total_size)
        }
        DramType::LPDDR3 => unimplemented!(),
        DramType::LPDDR4 => unimplemented!(),
        DramType::HBM => unimplemented!(),
        DramType::HBM2 => unimplemented!(),
    }
}

#[derive(Default)]
struct RingBuffer {
    sending_tasks: usize,
    received_tasks: usize,
}
impl RingBuffer {
    fn add_recieved_tasks(&mut self) {
        self.received_tasks += 1;
    }
    fn add_send_tasks(&mut self) {
        self.sending_tasks += 1;
    }
    /// return sending tasks and received tasks
    fn report_and_reset(&mut self) -> (usize, usize) {
        let ret = (self.sending_tasks, self.received_tasks);

        self.sending_tasks = 0;
        self.received_tasks = 0;
        ret
    }
}

#[derive(Clone)]
struct SubArray {
    read_open: Option<usize>,
    write_open: Option<(usize, usize)>,

    remote_write: Option<(usize, usize)>,

    sub_array_result: SubArrayResult,
    final_subarry_result: SubArrayResult,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SubArrayResult {
    /// total local cycle
    pub cycle: usize,
    /// for normal rows
    pub local_row_open_cycle: usize,
    /// row hit for read
    pub local_row_read_cycle: usize,
    /// row hit for write
    pub local_row_write_cycle: usize,
    /// cycle for compuation
    pub comp_cycle: usize,

    /// for evil row read/write miss
    pub local_row_open_cycle_evil: usize,
    /// row hit for read
    pub local_row_read_cycle_evil: usize,
    /// row hit for write
    pub local_row_write_cycle_evil: usize,

    // for remote rows that read by local subarray
    pub remote_row_read_cycle: usize,

    // remote result write by target subarray
    pub remote_row_write_cycle: usize,
    /// remote total cycle
    pub cycle_remote: usize,
}
impl SubArrayResult {
    fn accumulate(&self, other: &mut SubArrayResult) {
        other.cycle += self.cycle;
        other.local_row_open_cycle += self.local_row_open_cycle;
        other.local_row_read_cycle += self.local_row_read_cycle;
        other.local_row_write_cycle += self.local_row_write_cycle;
        other.comp_cycle += self.comp_cycle;
        other.local_row_open_cycle_evil += self.local_row_open_cycle_evil;
        other.local_row_read_cycle_evil += self.local_row_read_cycle_evil;
        other.local_row_write_cycle_evil += self.local_row_write_cycle_evil;
        other.remote_row_read_cycle += self.remote_row_read_cycle;
        other.remote_row_write_cycle += self.remote_row_write_cycle;
        other.cycle_remote += self.cycle_remote;
    }
    fn reset(&mut self) {
        self.cycle = 0;
        self.local_row_open_cycle = 0;
        self.local_row_read_cycle = 0;
        self.local_row_write_cycle = 0;
        self.comp_cycle = 0;
        self.local_row_open_cycle_evil = 0;
        self.local_row_read_cycle_evil = 0;
        self.local_row_write_cycle_evil = 0;
        self.remote_row_read_cycle = 0;
        self.remote_row_write_cycle = 0;
        self.cycle_remote = 0;
    }
}

impl SubArray {
    /// create a new subarray
    fn new() -> Self {
        Self {
            read_open: None,
            write_open: None,
            remote_write: None,
            sub_array_result: Default::default(),
            final_subarry_result: Default::default(),
        }
    }

    /// a local read and write task(local accumulate)
    fn add_task(
        &mut self,
        local_read: PhysicRowId,
        local_write: PhysicRowId,
        evil: bool,
        col_id: usize,
    ) {
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
                Some((last_write_row, last_write_col)) => {
                    if last_write_row == local_write.0 {
                        let write_cycle = if last_write_col > col_id {
                            last_write_col - col_id
                        } else {
                            col_id - last_write_col
                        };
                        self.sub_array_result.local_row_write_cycle_evil += write_cycle;
                        self.sub_array_result.cycle += write_cycle;
                        self.write_open = Some((last_write_row, col_id));
                    } else {
                        self.sub_array_result.local_row_open_cycle_evil += 19;
                        self.sub_array_result.cycle += 19;
                        self.write_open = Some((local_write.0, col_id));
                    }
                }
                _ => {
                    self.sub_array_result.local_row_open_cycle_evil += 9;
                    self.sub_array_result.cycle += 9;
                    self.write_open = Some((local_write.0, col_id));
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
                Some((last_write_row, last_write_col)) => {
                    if last_write_row == local_write.0 {
                        let write_cycle = if last_write_col > col_id {
                            last_write_col - col_id
                        } else {
                            col_id - last_write_col
                        };
                        self.sub_array_result.local_row_write_cycle += write_cycle;
                        self.sub_array_result.cycle += write_cycle;
                        self.write_open = Some((last_write_row, col_id));
                    } else {
                        self.sub_array_result.local_row_open_cycle += 19;
                        self.sub_array_result.cycle += 19;
                        self.write_open = Some((local_write.0, col_id));
                    }
                }
                _ => {
                    self.sub_array_result.local_row_open_cycle += 9;
                    self.sub_array_result.cycle += 9;
                    self.write_open = Some((local_write.0, col_id));
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
    fn add_remote_task(&mut self, local_write: PhysicRowId, col_id: usize) {
        match self.remote_write {
            Some((last_write_row, last_write_col)) => {
                if last_write_row == local_write.0 {
                    let shift_cycle = if col_id > last_write_col {
                        col_id - last_write_col
                    } else {
                        last_write_col - col_id
                    };
                    self.sub_array_result.remote_row_write_cycle += shift_cycle;
                    self.sub_array_result.cycle_remote += shift_cycle;
                    self.remote_write = Some((local_write.0, col_id));
                } else {
                    self.sub_array_result.remote_row_write_cycle += 19;
                    self.sub_array_result.cycle_remote += 19;
                    self.remote_write = Some((local_write.0, col_id));
                }
            }
            _ => {
                self.sub_array_result.remote_row_write_cycle += 9;
                self.sub_array_result.cycle_remote += 9;
                self.remote_write = Some((local_write.0, col_id));
            }
        }
    }

    fn report(&self) -> SubArrayResult {
        self.final_subarry_result.clone()
    }
    /// clear the current result and add them into the final result, return current local and remote
    fn report_current_round(&mut self) -> (usize, usize) {
        // first accumualte the result from sub_array_result to final_subarry_result
        let local_cycle = self.sub_array_result.cycle;
        let remote_cycle = self.sub_array_result.cycle_remote;
        self.sub_array_result
            .accumulate(&mut self.final_subarry_result);

        self.sub_array_result.reset();
        (local_cycle, remote_cycle)
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Ring {
    /// from port, next port, target (layer port)
    /// each port repersent a bank
    /// each bank have multiple subarrays
    /// Vec: Bank,Subarray,Tasks
    tasks: RingTasksInAllBanks,
    banks: usize,
    subarrays: usize,
    ring_result: RingResult,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct RingResult {
    pub cycle: usize,
    pub traffic: usize,
}
#[allow(dead_code)]
impl Ring {
    fn new(banks: usize, subarrays: usize) -> Self {
        Self {
            tasks: vec![vec![vec![]; subarrays]; banks],
            banks,
            subarrays,
            ring_result: Default::default(),
        }
    }
    /// add a task to the ring
    /// # The `local_subarray_id` is the subarray id of the local bank, not the global subarray id
    fn add_task(
        &mut self,
        local_subarray_id: usize,
        source: RingPort,
        next_port: RingPort,
        target: (RingId, RingPort),
    ) {
        self.tasks[source.0 as usize][local_subarray_id].push((source, next_port, target));
    }

    fn report(&self) -> RingResult {
        self.ring_result.clone()
    }

    fn report_current_round(&mut self) -> usize {
        // simulate the ring process
        let mut paths = vec![0; self.banks];
        for (source, next_port, (_target_layer, _target_port)) in
            self.tasks.iter().flatten().flatten()
        {
            let forward_len = (next_port.0 + self.banks as u8 - source.0) % self.banks as u8;
            let backward_len = (source.0 + self.banks as u8 - next_port.0) % self.banks as u8;
            let (from, to) = if forward_len < backward_len {
                (source.0, next_port.0)
            } else {
                (next_port.0, source.0)
            };
            for i in from..to {
                paths[i as usize] += 1;
            }
        }
        let current_round_cycle = *paths.iter().max().unwrap_or(&0);
        self.ring_result.cycle += current_round_cycle;
        self.ring_result.traffic += self.tasks.len();
        self.tasks.iter_mut().flatten().for_each(|x| x.clear());
        current_round_cycle
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Tsv {
    traffic: usize,
    tsv_result: TsvResult,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct TsvResult {
    pub cycle: usize,
    pub traffic: usize,
}
#[allow(dead_code)]

impl Tsv {
    fn new() -> Self {
        Self {
            traffic: 0,
            tsv_result: Default::default(),
        }
    }
    fn add_task(&mut self) {
        self.traffic += 1;
    }

    fn report(&self) -> TsvResult {
        self.tsv_result.clone()
    }
    fn report_current_round(&mut self) -> usize {
        self.tsv_result.cycle += self.traffic;
        self.tsv_result.traffic += self.traffic;
        let cycle = self.traffic;
        self.traffic = 0;
        cycle
    }
}

pub struct Hardware<'a, MP> {
    sub_array: Vec<SubArray>,
    ring: Vec<Ring>,
    tsv: Vec<Tsv>,
    ring_buffer: Vec<RingBuffer>,
    config: &'a ConfigV2,

    mapping: MP,
}
#[allow(dead_code)]
#[derive(Debug, Default)]
struct TsvReport {
    pub cycle: usize,
    pub cycle_no_conflict: usize,
    pub max_use: usize,
    pub max_use_valid: usize,
    pub real_use: usize,
}
#[allow(dead_code)]
#[derive(Debug, Default)]
struct TsvReportV2 {
    pub cycle_normal: usize,
    pub cycle_no_conflict: usize,
    pub max_use: usize,
    pub max_use_valid: usize,
    pub real_use: usize,
}

#[allow(dead_code)]
fn compute_result(mut tsv_traffic: Vec<VecDeque<&RingTask>>) -> TsvReport {
    let ports = tsv_traffic.len();
    let mut cycle = 0;
    let mut max_use = 0;
    let mut real_use = 0;
    // a cross bar net work
    let cycle_no_conflict = tsv_traffic.iter().map(|a| a.len()).max().unwrap_or(0);
    let mut shift = 0;
    let mut max_use_valid = 0;
    loop {
        let mut busy = false;

        let mut current_round_target = BTreeSet::new();
        let mut total_valid_ports = 0;
        let mut new_shift = shift;
        for i in 0..ports {
            let index = (i + shift) % ports;
            if let Some(traffic) = tsv_traffic[index].pop_front() {
                // valid one, select this one for next shift if it's not the current one
                if new_shift == shift {
                    new_shift = index;
                }
                total_valid_ports += 1;
                busy = true;
                if current_round_target.contains(&traffic.2 .0) {
                    tsv_traffic[index].push_front(traffic);
                } else {
                    current_round_target.insert(traffic.2 .0);
                }
            }
        }
        shift = new_shift;

        if !busy {
            break;
        } else {
            cycle += 1;
            max_use += ports;
            real_use += current_round_target.len();
            max_use_valid += total_valid_ports;
        }
    }

    TsvReport {
        cycle,
        max_use,
        real_use,
        cycle_no_conflict,
        max_use_valid,
    }
}
#[allow(dead_code)]

fn get_ring_interleave(rings_tasks: Vec<&RingTasksInAllBanks>) -> Vec<Vec<VecDeque<&RingTask>>> {
    rings_tasks
        .into_iter()
        .map(|r| {
            r.iter()
                .map(|bank| bank.iter().flat_interleave().collect())
                .collect()
        })
        .collect()
}
#[allow(dead_code)]
struct RingTraffic<T> {
    from: usize,
    to: usize,
    direction: Direction,
    traffic: T,
}
#[allow(dead_code)]

impl<T> RingTraffic<T> {
    fn new(from: usize, to: usize, direction: Direction, traffic: T) -> Self {
        Self {
            from,
            to,
            direction,
            traffic,
        }
    }
}
impl<T> IcntPacket for RingTraffic<T> {
    fn get_source(&self) -> usize {
        self.from
    }

    fn get_next_hop(&self) -> usize {
        self.to
    }

    fn get_direction(&self) -> Direction {
        self.direction
    }
}

#[allow(dead_code)]
struct CrossBarTraffic<T> {
    from: usize,
    to: usize,
    traffic: T,
}
#[allow(dead_code)]
impl<T> CrossBarTraffic<T> {
    fn new(from: usize, to: usize, traffic: T) -> Self {
        Self { from, to, traffic }
    }
}
impl<T> CrossBarPacket for CrossBarTraffic<T> {
    fn get_source(&self) -> usize {
        self.from
    }

    fn get_dest(&self) -> usize {
        self.to
    }
}

trait CrossBarCommon<T> {
    fn add(&mut self, node: usize, traffic: T) -> Result<(), T>;
    fn pop(&mut self, node: usize) -> Option<T>;
    fn cycle(&mut self);
}

impl<T: CrossBarPacket> CrossBarCommon<T> for CrossBareSimulator<T> {
    fn add(&mut self, node: usize, traffic: T) -> Result<(), T> {
        self.add(node, traffic)
    }

    fn pop(&mut self, node: usize) -> Option<T> {
        self.pop(node)
    }

    fn cycle(&mut self) {
        self.cycle()
    }
}
impl<T: CrossBarPacket> CrossBarCommon<T> for CrossBareSimulatorNoConflict<T> {
    fn add(&mut self, node: usize, traffic: T) -> Result<(), T> {
        self.add(node, traffic)
    }

    fn pop(&mut self, node: usize) -> Option<T> {
        self.pop(node)
    }

    fn cycle(&mut self) {
        self.cycle()
    }
}

impl<'a, MP: Mapping> Hardware<'a, MP> {
    #[allow(dead_code)]
    fn get_tsv_interleave(&self) -> Vec<VecDeque<&RingTask>> {
        let tsv_traffic = self.ring.iter().enumerate().map(|(ring_id, r)| {
            r.tasks
                .iter()
                .flat_interleave()
                .flat_interleave()
                .filter(move |d| d.2 .0 .0 != ring_id)
        });
        // now we got the remote traffic from ring to base layer, then we should make a detailed simulation to calculate the cycle
        let tsv_traffic: Vec<VecDeque<_>> = tsv_traffic.map(|t| t.collect()).collect();
        tsv_traffic
    }

    /// # situation
    /// - when a round is finished, all traffic from each input port are ready to route to the other port
    /// - the traffic is routed by a crossbar network in the base layer
    /// # inbanlance reason
    /// - the input traffic is not balanced
    /// - the output traffic is not balanced
    /// # solution
    /// - need a score to represent the reason why usage is low
    /// - the input reason
    /// - the output reason
    /// # TODO
    /// - redesign the buffer to store the traffic from each subarray. then use the [flat_interleave] to
    ///  route the traffic
    #[allow(dead_code)]
    fn calculate_tsv_traffic(&self) -> TsvReportV2 {
        // the input traffic from each bank in the layer

        // the old method is deprecated
        // `compute_result(self.get_tsv_interleave())`

        // first get the traffic for each bank and each ring
        let rings_tasks = self.ring.iter().map(|r| &r.tasks).collect_vec();
        let ring_bank_traffic: Vec<Vec<VecDeque<&RingTask>>> = get_ring_interleave(rings_tasks);
        // second, build the hardware:
        // - the ring simulator for each ring
        let ports = self.config.channels.num;
        let cycle_normal = {
            self.calculate_icnt(
                ring_bank_traffic.clone(),
                CrossBareSimulator::new(ports, 16),
            )
        };
        let cycle_no_conflict = {
            self.calculate_icnt(
                ring_bank_traffic,
                CrossBareSimulatorNoConflict::new(ports, 16),
            )
        };
        // println!("cycle: {}", cycle);
        tracing::debug!("cycle: {cycle_normal} {cycle_no_conflict}");
        TsvReportV2 {
            cycle_normal,
            cycle_no_conflict,
            max_use: 0,
            max_use_valid: 0,
            real_use: 0,
        }
    }

    fn calculate_icnt<Cross>(
        &self,
        mut ring_bank_traffic: Vec<Vec<VecDeque<&'a RingTask>>>,
        mut crossbar_simulator: Cross,
    ) -> usize
    where
        Cross: CrossBarCommon<CrossBarTraffic<&'a (RingPort, RingPort, (RingId, RingPort))>>,
    {
        // - the corssbar simulator for the base layer
        let mut ring_simulators = (0..self.config.channels.num)
            .map(|_ring_id| RingSimulator::new(self.config.banks.num, 128))
            .collect_vec();

        let mut tsv_buffer = (0..self.config.channels.num)
            .map(|_| VecDeque::with_capacity(16))
            .collect_vec();

        let mut total_remain_traffic = 0;
        let mut cycle = 0;
        loop {
            for (ring_id, (ring_sim, ring_traffic)) in ring_simulators
                .iter_mut()
                .zip(ring_bank_traffic.iter_mut())
                .enumerate()
            {
                for (bank_id, bank_traffic) in ring_traffic.iter_mut().enumerate() {
                    // first add the traffic to the input of the ring

                    if let Some(traffic) = bank_traffic.pop_front() {
                        // add the traffic to ring port  `bank_id`
                        let from = traffic.0 .0 as usize;
                        let to = traffic.1 .0 as usize;
                        let nodes = self.config.banks.num;
                        let right_distance = (to + nodes - from) % nodes;
                        let left_distance = (from + nodes - to) % nodes;
                        let direction = if right_distance < left_distance {
                            Direction::Right
                        } else {
                            Direction::Left
                        };
                        match ring_sim.add(bank_id, RingTraffic::new(from, to, direction, traffic))
                        {
                            Ok(_) => {
                                total_remain_traffic += 1;
                            }
                            Err(RingTraffic { traffic, .. }) => {
                                bank_traffic.push_front(traffic);
                            }
                        }
                    }
                    // then pop the traffic from the output of the ring

                    if let Some(RingTraffic { traffic, .. }) = ring_sim.front(bank_id) {
                        if bank_id == 0 {
                            // the tsv port
                            // test if the traffic should go to other ring
                            let target_ring_id = traffic.2 .0 .0;
                            if ring_id == target_ring_id {
                                // no!
                                // do nothing and pop the traffic
                                ring_sim.pop(bank_id);
                                total_remain_traffic -= 1;
                            } else {
                                // yes, go to the target ring
                                // push it to the tsv buffer
                                if tsv_buffer[ring_id].len() < 16 {
                                    let traffic = ring_sim.pop(bank_id).unwrap();
                                    tsv_buffer[ring_id].push_back(traffic);
                                    // do not need to decrease the total_remain_traffic because we will do it on pop of the crossbar
                                } else {
                                    // the buffer is full, do nothing
                                }
                            }
                        } else {
                            // just pop the traffic
                            ring_sim.pop(bank_id).unwrap();
                            total_remain_traffic -= 1;
                        }
                    }
                }
            }

            // handle the tsv buffer traffic
            for (tsv_id, tsv) in tsv_buffer.iter_mut().enumerate() {
                if let Some(traffic) = tsv.pop_front() {
                    let crossbare_packet =
                        CrossBarTraffic::new(tsv_id, traffic.traffic.2 .0 .0, traffic.traffic);
                    match crossbar_simulator.add(tsv_id, crossbare_packet) {
                        Ok(_) => {
                            // do nothing
                        }
                        Err(_) => {
                            tsv.push_front(traffic);
                        }
                    }
                }
                // handle the crossbar traffic out
                if let Some(_p) = crossbar_simulator.pop(tsv_id) {
                    // the packet is finished
                    total_remain_traffic -= 1;
                }
            }
            // cycle the simulators
            for ring_sim in ring_simulators.iter_mut() {
                ring_sim.cycle();
            }
            crossbar_simulator.cycle();

            cycle += 1;
            if total_remain_traffic == 0 {
                break;
            }
        }
        cycle
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
    /// - if the target is the same, just distribute the local
    /// - if the target is different, read the local and write merged data to the remote
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
        for (mat_b_row_id, col_id) in row_id_col_id {
            // noticed here, the evil col
            let partition_id = self.mapping.get_partition_id_row(mat_b_row_id);
            let target_partition_id = self.mapping.get_partition_id_col(col_id);
            if partition_id == target_partition_id {
                self.distribute_local(target_row_id, mat_b_row_id, col_id);
            } else {
                // record the remote tasks
                // first read the local row
                let physic_row_id = self.mapping.get_row_id(mat_b_row_id, col_id);
                self.sub_array[partition_id.0].add_remote_read_task(physic_row_id);
                // then store the temporary result for the remote dense
                *remote_tasks
                    .entry(partition_id)
                    .or_insert(BTreeMap::new())
                    .entry(col_id)
                    .or_insert(0) += 1;
            }
        }

        // send the remote to ring, tsv and remote subarray,
        // for each col id, there should only be one remote task(redundent remote col task are merged)
        for (subarray_id, subarray_entry) in remote_tasks {
            for (col_id, _entry) in subarray_entry {
                // let count = entry.1;
                assert_ne!(subarray_id, self.mapping.get_partition_id_col(col_id));
                self.distribute_remote(target_row_id, subarray_id, col_id);
            }
        }
    }

    fn distribute_local_evil_row(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.mapping.get_partition_id_col(col_id);
        let local_read = self.mapping.get_row_id_evil(mat_b_row_id, col_id);
        let local_write = self.mapping.get_row_id_dense(target_row_id, col_id);
        let local_col_id = self.mapping.get_col_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0].add_task(local_read, local_write, true, local_col_id);
    }

    fn distribute_local(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.mapping.get_partition_id_row(mat_b_row_id);
        let local_read = self.mapping.get_row_id(mat_b_row_id, col_id);
        let local_write = self.mapping.get_row_id_dense(target_row_id, col_id);
        let local_col_id = self.mapping.get_col_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0].add_task(local_read, local_write, false, local_col_id);
    }

    fn read_local_and_distribute_remote(
        &mut self,
        target_row_id: LogicRowId,
        subarray_id: SubarrayId,
        local_subarray: SubarrayId,
        matrix_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // read the local row
        let local_read_row_id = self.mapping.get_row_id(matrix_b_row_id, col_id);
        self.sub_array[local_subarray.0].add_remote_read_task(local_read_row_id);
        self.distribute_remote(target_row_id, subarray_id, col_id);
    }

    /// this task will have to write the result to a remote subarray
    fn distribute_remote(
        &mut self,
        target_row_id: LogicRowId,
        _subarray_id: SubarrayId,
        col_id: LogicColId,
    ) {
        // ignore the ring and tsv here cause it's overlaped with local accumulation.
        let target_partition_id = self.mapping.get_partition_id_col(col_id);
        let remote_dense_row_write = self.mapping.get_row_id_dense(target_row_id, col_id);
        let col_id = self.mapping.get_col_id_dense(target_row_id, col_id);
        self.sub_array[target_partition_id.0].add_remote_task(remote_dense_row_write, col_id);
        let target_bank_id = self.mapping.ring_buffer_id(target_partition_id);
        let self_bank_id = self.mapping.ring_buffer_id(_subarray_id);
        self.ring_buffer[target_bank_id.0].add_recieved_tasks();
        self.ring_buffer[self_bank_id.0].add_send_tasks();
    }
}

impl<'a, MP> Hardware<'a, MP> {
    fn new(config: &'a ConfigV2, mapping: MP) -> Self {
        // each single layer should be a channel
        let banks = config.channels.num * config.banks.num;
        let num_subarray = banks * config.subarrays;
        let num_rings = config.channels.num;
        let num_tsvs = config.channels.num;
        Self {
            sub_array: vec![SubArray::new(); num_subarray],
            ring: vec![Ring::new(config.banks.num, config.subarrays); num_rings],
            tsv: vec![Tsv::new(); num_tsvs],
            ring_buffer: (0..banks).map(|_| RingBuffer::default()).collect(),
            config,

            mapping,
        }
    }

    /// reduce the result and return the result
    fn report(
        &self,
        name: String,
        total_result: GlobalStatV2,
        batch: usize,
        topk: f32,
    ) -> SingleResult {
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
            total_result,
            batch,
            topk,
        }
    }
}
pub struct GearboxSim<'a, 'b, MP> {
    pub row_per_partition: usize,
    #[allow(unused)]
    pub col_per_partition: usize,
    /// the id of evil col: this col will  have a backup copy in each partition
    pub evil_col_ids: HashSet<usize>,
    /// the id of evil row: the evil row will be partitioned into each components,there are no remote access needed.
    pub evil_row_ids: HashSet<usize>,
    pub matrix_b: &'a CsMatI<Pattern, u32>,
    pub hardware: Hardware<'b, MP>,
}

impl<'a, 'b, MP: Mapping> GearboxSim<'a, 'b, MP> {
    /// distribute the task to components
    ///
    /// we should analyze the gearbox overflow overhead and the icnt unbanlance traffic in this
    /// version
    fn run(
        &mut self,
        input_vec: &CsMatI<Pattern, u32>,
        current_batch: usize,
        _current_topk: f32,
    ) -> GlobalStatV2 {
        let mut global_stats = GlobalStatV2::default();
        let now = std::time::Instant::now();
        debug!("run gearbox sim");
        let evil_rows = self.evil_row_ids.len();
        let evil_cols = self.evil_col_ids.len();
        debug!(?self.row_per_partition,?self.row_per_partition,  ?evil_rows, ?evil_cols, "run gearbox sim");
        debug!(?self.evil_row_ids, ?self.evil_col_ids, "run gearbox sim");
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
            let mut evil_col_row_id_col_id = vec![];

            // get the result for that line
            for &mat_b_row_id in row.indices() {
                let mat_b_row_id = mat_b_row_id as usize;
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
                            .map(|&x| LogicColId(x as usize)),
                    );
                } else {
                    // the row is not evil, need to access remote
                    for col in self
                        .matrix_b
                        .outer_view(mat_b_row_id)
                        .unwrap()
                        .indices()
                        .iter()
                        .map(|i| *i as usize)
                    {
                        if self.evil_col_ids.contains(&col) {
                            // the col is evil, no need to access remote
                            // self.hardware
                            //     .distribute_evil_col(target_id, *mat_b_row_id, *col);
                            evil_col_row_id_col_id
                                .push((LogicRowId(mat_b_row_id), LogicColId(col)));
                        } else {
                            // the col is not evil, need to access remote
                            let target_partition =
                                self.hardware.mapping.get_partition_id_col(LogicColId(col));
                            let source_partition = self
                                .hardware
                                .mapping
                                .get_partition_id_row(LogicRowId(mat_b_row_id));
                            if target_partition == source_partition {
                                self.hardware.distribute_local(
                                    LogicRowId(target_id),
                                    LogicRowId(mat_b_row_id),
                                    LogicColId(col),
                                );
                            } else {
                                // the col is in different partition, need to access remote
                                self.hardware.read_local_and_distribute_remote(
                                    LogicRowId(target_id),
                                    self.hardware
                                        .mapping
                                        .get_partition_id_row(LogicRowId(mat_b_row_id)),
                                    self.hardware
                                        .mapping
                                        .get_partition_id_row(LogicRowId(mat_b_row_id)),
                                    LogicRowId(mat_b_row_id),
                                    LogicColId(col),
                                );
                            }
                        }
                    }
                }
            }
            // fix bug here, we should collect the evil col for each target id
            self.hardware
                .distribute_evil_col(LogicRowId(target_id), evil_col_row_id_col_id);
            // reduce the tasks and clear the tasks
            // the cycle of this round
            if (target_id + 1) % current_batch == 0 {
                update_stats(&mut self.hardware, &mut global_stats);
                // the data for overflow:
            }
            // add the result to the total result and continue to the next line
        }
        global_stats
    }
}

impl<'a, 'b, MP> GearboxSim<'a, 'b, MP> {
    fn new(
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: &'a CsMatI<Pattern, u32>,
        config: &'b ConfigV2,
        mapping: MP,
    ) -> Self {
        let num_partitions = config.channels.num * config.banks.num * config.subarrays;
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

        GearboxSim {
            row_per_partition,
            col_per_partition,
            evil_col_ids: evil_col_ids.into_iter().collect(),
            evil_row_ids: evil_row_ids.into_iter().collect(),
            matrix_b,
            hardware: Hardware::new(config, mapping),
        }
    }

    /// reduce the result and return the result
    fn report(
        &self,
        name: String,
        total_result: GlobalStatV2,
        batch: usize,
        topk: f32,
    ) -> SingleResult {
        self.hardware.report(name, total_result, batch, topk)
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalStat {
    pub global_tsv_base_total: usize,
    pub global_tsv_base_real: usize,
    pub global_tsv_base_cycle: usize,
    pub global_tsv_base_cycle_no_conflict: usize,
    pub global_tsv_base_max_use_validt: usize,
    pub overflow_count_8_256: usize,
    pub overflow_count_8_256_overhead: usize,
    pub overflow_count_8_512: usize,
    pub overflow_count_8_512_overhead: usize,
    pub overflow_count_12_256: usize,
    pub overflow_count_12_256_overhead: usize,
    pub overflow_count_12_512: usize,
    pub overflow_count_12_512_overhead: usize,
    pub total_counts: i32,
    pub global_max_acc_cycle: usize,
    pub global_max_acc_cycle_remote: usize,
    pub global_max_acc_ring: usize,
    pub global_max_acc_tsv: usize,
    pub global_max_real_local: usize,
    pub global_max_ring_buffer: usize,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalStatV2 {
    pub global_tsv_base_total: usize,
    pub global_tsv_base_real: usize,
    pub global_tsv_base_cycle_normal: usize,
    pub global_tsv_base_cycle_no_conflict: usize,
    pub global_tsv_base_max_use_validt: usize,
    pub overflow_count_8_256: usize,
    pub overflow_count_8_256_overhead: usize,
    pub overflow_count_8_512: usize,
    pub overflow_count_8_512_overhead: usize,
    pub overflow_count_12_256: usize,
    pub overflow_count_12_256_overhead: usize,
    pub overflow_count_12_512: usize,
    pub overflow_count_12_512_overhead: usize,
    pub total_counts: i32,
    pub global_max_acc_cycle: usize,
    pub global_max_acc_cycle_remote: usize,
    pub global_max_acc_ring: usize,
    pub global_max_acc_tsv: usize,
    pub global_max_real_local: usize,
    pub global_max_ring_buffer: usize,
    pub global_max_dispatching: usize,

    pub top_1000_distribution: BTreeSet<SortedDistribution>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SortedDistribution {
    pub total_cycle: usize,

    pub local_max: usize,
    pub local_average: f64,
    pub local_variance: f64,

    pub local_dispatcher_max: usize,
    pub local_dispatcher_average: f64,
    pub local_dispatcher_variance: f64,
    pub local_dispatcher_50: f64,
    pub local_dispatcher_75: f64,

    pub dispatching_max: usize,
    pub dispatching_average: f64,
    pub dispatching_variance: f64,
    pub dispatching_50: f64,
    pub dispatching_75: f64,

    pub remote_max: usize,
    pub remote_average: f64,
    pub remote_variance: f64,
    pub remote_50: f64,
    pub remote_75: f64,
}
impl Eq for SortedDistribution {}

impl PartialOrd for SortedDistribution {
    fn partial_cmp(&self, other: &Self) -> std::option::Option<std::cmp::Ordering> {
        self.total_cycle.partial_cmp(&other.total_cycle)
    }
}

impl Ord for SortedDistribution {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.total_cycle.cmp(&other.total_cycle)
    }
}

/// we need to get more infomation:
///
/// 1. the distribution of the local
/// 2. the distribution of the local dispatcher
/// 3. the distribution of the dispatching
/// 4. the distribution of the remote update
fn update_stats<T>(hardware: &mut Hardware<T>, global_stats: &mut GlobalStatV2) {
    // test if the size is overflow!
    // let tsv_report_base = hardware.calculate_tsv_traffic();
    // global_stats.global_tsv_base_total += tsv_report_base.max_use;
    // global_stats.global_tsv_base_real += tsv_report_base.real_use;
    // global_stats.global_tsv_base_cycle_normal += tsv_report_base.cycle_normal;
    // global_stats.global_tsv_base_cycle_no_conflict += tsv_report_base.cycle_no_conflict;
    // global_stats.global_tsv_base_max_use_validt += tsv_report_base.max_use_valid;
    // let ring_max_cycle = hardware
    //     .ring
    //     .iter_mut()
    //     .map(|ring| ring.report_current_round())
    //     .max()
    //     .unwrap();
    // let tsv_max_cycle = hardware
    //     .tsv
    //     .iter_mut()
    //     .map(|tsv| tsv.report_current_round())
    //     .max()
    //     .unwrap();
    // get the max ring buffer cycle and count for the overflow
    // received and send, only received
    let ring_buffer_max: (Vec<_>, Vec<_>) = hardware
        .ring_buffer
        .iter_mut()
        .map(|ring_buffer| {
            // in this function, we need to calculate the overhead of the overflow
            //
            // when an overflow happenend we need to stop all other subarrays and
            // finish that write, the cycle will be composed by:
            // 1. send from the original subarray the cycle should be the total cycle
            //    of the overflow
            // 2. others should be ignored for now
            let ring_buffer_cycle = ring_buffer.report_and_reset();
            // the frist one is the cycle of local stage,the second one is the cycle of remote stage
            (
                ring_buffer_cycle.0 + ring_buffer_cycle.1,
                ring_buffer_cycle.1,
            )
        })
        .unzip();
    let dispatcher_local_average =
        ring_buffer_max.0.iter().sum::<usize>() as f64 / ring_buffer_max.0.len() as f64;
    let dispatcher_local_variance = ring_buffer_max
        .0
        .iter()
        .map(|x| *x as f64)
        .population_variance();
    let dispatcher_local = ring_buffer_max.0.iter().max().unwrap();
    let dispatcher_remote_average =
        ring_buffer_max.1.iter().sum::<usize>() as f64 / ring_buffer_max.1.len() as f64;
    let dispatcher_remote_variance = ring_buffer_max
        .1
        .iter()
        .map(|x| *x as f64)
        .population_variance();

    let dispatcher_remote = ring_buffer_max.1.iter().max().unwrap();

    // the subarray max cycle for local
    let (max_local, max_remote): (Vec<_>, Vec<_>) = hardware
        .sub_array
        .iter_mut()
        .map(|sub_array| {
            let (local, remote) = sub_array.report_current_round();
            (local, remote)
        })
        .unzip();
    let max_local_cycle = max_local.iter().max().unwrap();
    let max_remote_cycle = max_remote.iter().max().unwrap();
    let max_local_average = max_local.iter().sum::<usize>() as f64 / max_local.len() as f64;
    let max_local_variance = max_local.iter().map(|x| *x as f64).population_variance();
    let max_remote_average = max_remote.iter().sum::<usize>() as f64 / max_remote.len() as f64;
    let max_remote_variance = max_remote.iter().map(|x| *x as f64).population_variance();

    let max_real_local_cycle = max_local_cycle.max(dispatcher_local);

    let distribution_stats = SortedDistribution {
        total_cycle: max_real_local_cycle + dispatcher_remote + max_remote_cycle,
        local_max: *max_local_cycle,
        local_average: max_local_average,
        local_variance: max_local_variance,
        local_dispatcher_max: *dispatcher_local,
        local_dispatcher_average: dispatcher_local_average,
        local_dispatcher_variance: dispatcher_local_variance,
        dispatching_max: *dispatcher_remote,
        dispatching_average: dispatcher_remote_average,
        dispatching_variance: dispatcher_remote_variance,
        remote_max: *max_remote_cycle,
        remote_average: max_remote_average,
        remote_variance: max_remote_variance,
        ..Default::default()
    };
    global_stats.global_max_acc_cycle += max_local_cycle;
    global_stats.global_max_acc_cycle_remote += max_remote_cycle;
    global_stats.global_max_real_local += max_real_local_cycle;
    global_stats.global_max_ring_buffer += *dispatcher_local;
    global_stats.global_max_dispatching += *dispatcher_remote;
    global_stats
        .top_1000_distribution
        .insert(distribution_stats);
    // there should be at max 1000 entries in the top 1000 distribution, delete one if it is full
    if global_stats.top_1000_distribution.len() > 1024 {
        global_stats.top_1000_distribution.pop_first();
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

fn compute_gearbox(config: &ConfigV2, path: &str) -> Vec<SingleResult> {
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
    let matrix_head: MatrixHead<Pattern, u32> = sprs::io::read_matrix_market_head(path).unwrap();
    let matrix_size = matrix_head.ind_ptr_size() + matrix_head.ind_size() + matrix_head.data_size();
    // two csr csc matrix during runtime
    let matrix_size = matrix_size * 2;
    let sim_size = partitions * (size_of::<SubArray>()) * 2;
    let sim_size = matrix_size + sim_size;

    let temp_size = matrix_head.tri_size();

    info!(
        "info there will be {} bytes,start acquire the space",
        sim_size + temp_size
    );
    let mut _guard = crate::acquire_memory_sections(&[sim_size, temp_size]);
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
            let mapping = same_subarray::SameSubarrayMapping::new(
                config,
                row_per_partition,
                col_per_partition,
            );
            let mut gearbox = GearboxSim::new(
                mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
                mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
                &matrix_b,
                config,
                mapping,
            );
            info!("start running the sim");
            let result = gearbox.run(&matrix_a, batch, top_k);
            TOTAL_FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            info!(
                "finished task: {}/{}",
                TOTAL_FINISHED_TASKS.load(Ordering::Relaxed),
                *TOTAL_TASKS.read().unwrap()
            );
            gearbox.report(path.to_string(), result, batch, top_k)
        })
        .collect();
    drop(matrix_a);
    drop(matrix_b);
    drop(_guard_sim);
    results
}
pub fn transpose2<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

static TOTAL_FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_TASKS: RwLock<usize> = RwLock::new(0);

fn analyze_gearbox_inner<LevelType: LevelTrait>(
    config: &ConfigV2,
    _total_size: &LevelType::Storage,
) -> Vec<((usize, f32), Vec<SingleResult>)>
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
            compute_gearbox(config, path)
        })
        .collect();
    let results = transpose2(results);
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
    // GearboxResult { results }
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
