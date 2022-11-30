//! helper functions for computing the merger cycle

use crate::pim::{level::LevelTrait, task_manager::RoundTasks};

pub fn compute_finished_time_for_single_task<LevelType: LevelTrait>(
    task: &RoundTasks<LevelType>,
    total_size: &LevelType,
) -> u64 {
    todo!()
}

pub fn compute_finished_time_for_tasks<LevelType: LevelTrait>(
    tasks: &[RoundTasks<LevelType>],
    total_size: &LevelType,
) -> Vec<u64> {
    todo!()
}
