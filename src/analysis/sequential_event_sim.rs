//! the sequential event simulator

use std::{cmp::Reverse, collections::BinaryHeap};

use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
    task::Task,
    task_manager::{self, GraphATasks},
    SimulationContext,
};

/// the events
enum EventType<LevelType: LevelTrait> {
    /// one task is finished, should try the next one
    TaskFinished(Task<LevelType>),
    Start,
}

struct Event<LevelType: LevelTrait> {
    finished_time: u64,
    event: EventType<LevelType>,
}

impl<LevelType: LevelTrait> PartialEq for Event<LevelType> {
    fn eq(&self, other: &Self) -> bool {
        self.finished_time == other.finished_time
    }
}
impl<LevelType: LevelTrait> Eq for Event<LevelType> {}
impl<LevelType: LevelTrait> PartialOrd for Event<LevelType> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<LevelType: LevelTrait> Ord for Event<LevelType> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.finished_time.cmp(&self.finished_time)
    }
}

/// the event driven lock
#[derive(Debug)]
pub struct Statistics {}

pub fn compute_lock_task_overlap_stat(config: &Config) -> Statistics {
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
            compute_lock_task_overlap_stat_inner::<ddr4::Level>(config, total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

/// the dram resource that allocated by tasks
struct ResourceConstraint<LevelType: LevelTrait> {
    total_size: LevelType::Storage,
}

impl<LevelType: LevelTrait> ResourceConstraint<LevelType> {
    fn new(total_size: LevelType::Storage) -> Self {
        Self { total_size }
    }
    /// allocate the resource for a task,
    /// return true if the allocation is successful
    fn allocate(
        &mut self,
        task: Task<LevelType>,
        current_time: u64,
    ) -> Result<Event<LevelType>, Task<LevelType>> {
        todo!()
    }
    // free the resource for a task
    fn deallocate(&mut self, task: Task<LevelType>) {
        todo!()
    }
}
fn schedule_new_tasks<LevelType: LevelTrait>(
    graph_a_iter: &mut task_manager::IntoIter<LevelType>,
    current_waiting_task: &mut Option<Task<LevelType>>,
    event_queue: &mut BinaryHeap<Reverse<Event<LevelType>>>,
    resource_constraint: &mut ResourceConstraint<LevelType>,
    current_time: u64,
) {
    if let Some(task) = current_waiting_task.take() {
        // try to allocate the resource
        match resource_constraint.allocate(task, current_time) {
            Ok(event) => {
                event_queue.push(Reverse(event));
            }
            Err(task) => {
                *current_waiting_task = Some(task);
                return;
            }
        }
    }
    assert!(current_waiting_task.is_none());
    // try to find the next task
    while let Some(task) = graph_a_iter.next() {
        // try to allocate the resource
        match resource_constraint.allocate(task, current_time) {
            Ok(event) => {
                event_queue.push(Reverse(event));
            }
            Err(task) => {
                *current_waiting_task = Some(task);
                break;
            }
        }
    }
}

pub fn compute_lock_task_overlap_stat_inner<LevelType: LevelTrait>(
    config: &Config,
    total_size: LevelType::Storage,
) -> Statistics
where
    LevelType::Storage: Ord,
{
    // first build the tasks
    let graph_path = &config.graph_path;
    let graph_a = sprs::io::read_matrix_market(graph_path).unwrap().to_csr();
    let graph_b = graph_a.transpose_view().to_csr();
    let graph_b_mapping = LevelType::get_mapping(&total_size, &graph_b);

    let mut context = SimulationContext::new(config);
    let graph_a_tasks: GraphATasks<LevelType> =
        GraphATasks::generate_mappings_for_a(&graph_a, &graph_b_mapping, &mut context);

    println!("graph_a_tasks: {:?}", graph_a_tasks.tasks.len());
    let mut event_queue: BinaryHeap<Reverse<Event<LevelType>>> = BinaryHeap::new();
    event_queue.push(Reverse(Event {
        finished_time: 0,
        event: EventType::Start,
    }));
    let mut resource_constraint: ResourceConstraint<LevelType> =
        ResourceConstraint::new(total_size);
    let mut graph_a_iter = graph_a_tasks.into_iter();
    let mut current_waiting_task = None;
    while let Some(Reverse(event)) = event_queue.pop() {
        match event.event {
            EventType::TaskFinished(finished_task) => {
                resource_constraint.deallocate(finished_task);
                schedule_new_tasks(
                    &mut graph_a_iter,
                    &mut current_waiting_task,
                    &mut event_queue,
                    &mut resource_constraint,
                    event.finished_time,
                );
            }
            EventType::Start => {
                // schedule new tasks
                schedule_new_tasks(
                    &mut graph_a_iter,
                    &mut current_waiting_task,
                    &mut event_queue,
                    &mut resource_constraint,
                    event.finished_time,
                );
            }
        }
    }
    Statistics {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lock() {
        let config: Config =
            toml::from_str(std::fs::read_to_string("ddr4.toml").unwrap().as_str()).unwrap();
        println!("{:?}", config);
        let stat = compute_lock_task_overlap_stat(&config);
        println!("{:?}", stat);
    }

    #[test]
    fn test_list() {
        let mut list: Vec<_> = (0..=99).collect();
        list.sort_by_key(|&i| (i / 10, Reverse(i)));
        println!("{:?}", list);
    }
}
