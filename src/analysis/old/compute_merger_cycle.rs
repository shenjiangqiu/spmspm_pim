//! helper functions for computing the merger cycle

use crate::pim::{level::LevelTrait, task_manager::RoundTasks};
/// TODO!
pub fn compute_finished_time_for_single_task<LevelType: LevelTrait>(
    _task: &RoundTasks<LevelType>,
    _total_size: &LevelType,
) -> u64 {
    todo!()
}

/// TODO!
pub fn compute_finished_time_for_tasks<LevelType: LevelTrait>(
    _tasks: &[RoundTasks<LevelType>],
    _total_size: &LevelType,
) -> Vec<u64> {
    todo!()
}
