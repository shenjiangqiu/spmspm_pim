//! create a method to schedule multple tasks that in a window event they have confilicts
//! - inside the window, when they have conflicts, they will try to share the stream
//! - outside the window, they will try to use the duplicated stream
//! - otherwise, they will try to run partial merger
#![allow(unused)]
use std::{
    cmp::Reverse,
    collections::{BTreeSet, BinaryHeap},
};

use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
    task_manager::{self, GraphATasks, RoundTasks},
    SimulationContext,
};

pub struct MergeredStreamStat {}

pub fn mergered_stream(config: &Config) -> Vec<MergeredStreamStat> {
    match config.dram_type {
        crate::pim::config::DramType::DDR3 => todo!(),
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
            mergered_stream_inner::<ddr4::Level>(config, total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

enum EventType<LevelType: LevelTrait> {
    SubarrayFinished(RoundTasks<LevelType>),
    Started,
}

type Event<LevelType> = super::event::Event<EventType<LevelType>>;

/// the dram resource that allocated by tasks
struct ResourceConstraint<LevelType: LevelTrait> {
    total_size: LevelType::Storage,
    sub_array_occupied: BTreeSet<usize>,
}

impl<LevelType: LevelTrait> ResourceConstraint<LevelType> {
    fn new(total_size: LevelType::Storage) -> Self {
        let sub_array_occupied = BTreeSet::new();
        Self {
            total_size,
            sub_array_occupied,
        }
    }
    /// allocate the resource for a task,
    /// return true if the allocation is successful
    fn allocate(
        &mut self,
        task: RoundTasks<LevelType>,
        current_time: u64,
    ) -> Result<Vec<Event<LevelType>>, RoundTasks<LevelType>> {
        // let mut events = vec![];
        // try to allocate the subarrays that are not occupied
        if task.tasks.iter().any(|task| {
            let sub_array_id = LevelType::get_flat_level_id(
                &LevelType::last_level(),
                &self.total_size,
                &task.target_id.level_path,
            );
            self.sub_array_occupied.contains(&sub_array_id)
        }) {
            // the task cannot be allocated
            return Err(task);
        }
        // can be allocated
        for task in task.tasks.iter() {
            let sub_array_id = LevelType::get_flat_level_id(
                &LevelType::last_level(),
                &self.total_size,
                &task.target_id.level_path,
            );
            self.sub_array_occupied.insert(sub_array_id);
        }
        // compute the finished time
        let cycle: u64 = task.tasks.iter().map(|task| (task.size / 4) as u64).sum();
        let finished_time = current_time + cycle;
        todo!();
    }
    /// free the resource for a task
    fn deallocate(&mut self, task: RoundTasks<LevelType>) {
        for task in task.tasks.iter() {
            let sub_array_id = LevelType::get_flat_level_id(
                &LevelType::last_level(),
                &self.total_size,
                &task.target_id.level_path,
            );
            self.sub_array_occupied.remove(&sub_array_id);
        }
    }
}

/// schedule a bunch of tasks
/// 1. in this window, if they can share the same subarray, they will share the same subarray
fn schedule_new_tasks<LevelType: LevelTrait>(
    graph_a_iter: &mut task_manager::IntoIterRound<LevelType>,
    current_waiting_task: &mut Option<Vec<RoundTasks<LevelType>>>,
    event_queue: &mut BinaryHeap<Reverse<Event<LevelType>>>,
    resource_constraint: &mut ResourceConstraint<LevelType>,
    current_time: u64,
    window_size: usize,
) {
    // if let Some(task) = current_waiting_task.take() {
    //     // try to allocate the resource
    //     match resource_constraint.allocate(task, current_time) {
    //         Ok(events) => {
    //             event_queue.extend(events.into_iter().map(|event| Reverse(event)));
    //         }
    //         Err(task) => {
    //             // not scheduled, put it back
    //             *current_waiting_task = Some(task);
    //             return;
    //         }
    //     }
    // }
    // assert!(current_waiting_task.is_none());
    // // try to find the next task
    // while let Some(task) = graph_a_iter.next() {
    //     // try to allocate the resource
    //     match resource_constraint.allocate(task, current_time) {
    //         Ok(event) => {
    //             event_queue.extend(event.into_iter().map(|event| Reverse(event)));
    //         }
    //         Err(task) => {
    //             *current_waiting_task = Some(task);
    //             break;
    //         }
    //     }
    // }
}

pub fn mergered_stream_inner<LevelType: LevelTrait>(
    config: &Config,
    total_size: LevelType::Storage,
) -> Vec<MergeredStreamStat>
where
    LevelType::Storage: Ord,
{
    let mut stat = vec![];
    for graph in &config.graph_path {
        println!("graph: {}", graph);
        // first build the tasks
        println!("parsing the graph");
        let graph_a = sprs::io::read_matrix_market(graph).unwrap().to_csr();
        let graph_b = graph_a.transpose_view().to_csr();
        let graph_b_mapping = LevelType::get_mapping(&total_size, &graph_b);
        // update the stats
        let total_sub_array = LevelType::get_total_level(&LevelType::last_level(), &total_size);
        let mut context = SimulationContext::new(config);
        let graph_a_tasks: GraphATasks<LevelType> =
            GraphATasks::generate_mappings_for_a(&graph_a, &graph_b_mapping, &mut context);

        println!("graph_a_tasks: {:?}", graph_a_tasks.tasks.len());
        let mut event_queue: BinaryHeap<Reverse<Event<LevelType>>> = BinaryHeap::new();
        event_queue.push(Reverse(Event {
            finished_time: 0,
            event: EventType::Started,
        }));
        let mut resource_constraint: ResourceConstraint<LevelType> =
            ResourceConstraint::new(total_size.clone());
        let mut graph_a_iter = graph_a_tasks.into_round_iter();
        let mut current_waiting_task = None;
        let mut final_time = 0;
        let mut last_stat_time = 0;
        let mut total_in_use_sub_array = 0;
        let mut total_idle_sub_array = 0;
        println!("start simulation");
        while let Some(Reverse(event)) = event_queue.pop() {
            final_time = event.finished_time;
            match event.event {
                EventType::SubarrayFinished(finished_task) => {
                    let time_elapsed = final_time - last_stat_time;
                    last_stat_time = final_time;

                    let occupied_sub_array = resource_constraint.sub_array_occupied.len();
                    let idle_sub_array = total_sub_array - occupied_sub_array;
                    total_in_use_sub_array += occupied_sub_array as u128 * time_elapsed as u128;
                    total_idle_sub_array += idle_sub_array as u128 * time_elapsed as u128;
                    // free the resource
                    resource_constraint.deallocate(finished_task);
                    // schedule the next task
                    schedule_new_tasks(
                        &mut graph_a_iter,
                        &mut current_waiting_task,
                        &mut event_queue,
                        &mut resource_constraint,
                        event.finished_time,
                        config.window_size,
                    );
                }
                EventType::Started => {
                    // schedule new tasks
                    schedule_new_tasks(
                        &mut graph_a_iter,
                        &mut current_waiting_task,
                        &mut event_queue,
                        &mut resource_constraint,
                        event.finished_time,
                        config.window_size,
                    );
                }
            }
        }
        stat.push(MergeredStreamStat {});
    }
    stat
}
