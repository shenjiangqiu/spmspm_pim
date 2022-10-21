use std::{
    collections::{BTreeSet, VecDeque},
    fmt::Debug,
    ops::RangeFull,
};

use sprs::{num_kinds::Pattern, CsMat};
use tracing::debug;

use super::{
    level::{self, GraphBRow, LevelTrait, MatrixBMapping, PathStorage},
    stream_merger::{EmptyComponent, StreamProvider, TaskReceiver},
    task::{PathId, StreamMessage, Task, TaskBuilder, TaskTo},
    Component, SimulationContext,
};

/// track the finish of tasks
/// - will init the final result
/// - when finished one taks, will update the final result
/// - when all tasks finished, will return the final result
#[derive(Debug)]
pub struct TaskManager<Child, Mapping: MatrixBMapping> {
    child: Child,
    graph_a_tasks: GraphATasks<Mapping::Storage>,
    unfinished_tasks: BTreeSet<TaskTo>,
    recent_to: Option<TaskTo>,
}

impl<Child: EmptyComponent, Mapping: MatrixBMapping> EmptyComponent
    for TaskManager<Child, Mapping>
{
    fn is_empty(&self) -> Vec<String> {
        let mut result = self.child.is_empty();
        if !self.unfinished_tasks.is_empty() {
            result.push("unfinished tasks".to_string());
        }
        result.extend(self.child.is_empty());
        result
    }
}

#[derive(Debug)]
pub struct GraphATasks<Storage> {
    current_working_target: usize,

    /// for each target, the froms
    tasks: VecDeque<VecDeque<Task<Storage>>>,
}

impl<Child, Mapping> TaskManager<Child, Mapping>
where
    Mapping: level::MatrixBMapping,
    Mapping::Storage: Clone + Ord,
{
    pub fn new(
        child: Child,
        graph_a: &CsMat<Pattern>,
        graph_b: &CsMat<Pattern>,
        total_size: &Mapping::Storage,
        task_builder: &mut TaskBuilder,
    ) -> Self {
        let graph_b_mappings = Mapping::get_mapping(total_size, graph_b);
        let graph_a_tasks = Self::generate_mappings_for_a(graph_a, &graph_b_mappings, task_builder);

        Self {
            child,
            graph_a_tasks,
            unfinished_tasks: BTreeSet::new(),
            recent_to: None,
        }
    }

    /// from graph a genrate a list of froms
    pub fn generate_mappings_for_a(
        graph_a: &CsMat<Pattern>,
        graph_b_mappings: &Mapping,
        task_builder: &mut TaskBuilder,
    ) -> GraphATasks<Mapping::Storage> {
        let mut tasks = VecDeque::new();
        for (to, row) in graph_a.outer_iterator().into_iter().enumerate() {
            let patches: Vec<(usize, &GraphBRow<Mapping::Storage>)> = row
                .indices()
                .iter()
                .map(|idx| {
                    let graph_b_row_detail = graph_b_mappings.get_row_detail(*idx);
                    (*idx, graph_b_row_detail)
                })
                .collect();

            let mut current_round = patches;
            let mut next_round = Vec::new();
            let mut current_round_num = 0;
            let mut this_target_tasks = VecDeque::new();
            while !current_round.is_empty() {
                let mut uniq_set = BTreeSet::new();
                let mut this_round_tasks = VecDeque::new();
                for (from, row_detail) in current_round.drain(RangeFull) {
                    if uniq_set.insert(PathStorage::get_sub_path_to_level(
                        &row_detail.path,
                        &LevelTrait::last_level(),
                    )) {
                        // yes , it's uniq
                        //generate the task
                        let task = task_builder.gen_task(
                            PathId::new(row_detail.path.clone()),
                            from,
                            TaskTo {
                                to,
                                round: current_round_num,
                            },
                            row_detail.size,
                        );
                        this_round_tasks.push_back(task);
                    } else {
                        // no, it's not uniq,
                        next_round.push((from, row_detail));
                    }
                }
                let end_task = task_builder.gen_end_task(TaskTo {
                    to,
                    round: current_round_num,
                });
                this_round_tasks.push_back(end_task);
                this_target_tasks.push_back(this_round_tasks);
                current_round = next_round.drain(RangeFull).collect();
                current_round_num += 1;
            }
            tasks.extend(this_target_tasks);
        }

        GraphATasks {
            current_working_target: 0,
            tasks,
        }
    }
}

impl<Child, LevelType: LevelTrait + Debug> Component for TaskManager<Child, LevelType::Mapping>
where
    LevelType::Storage: Clone,
    Child: StreamProvider<SimContext = SimulationContext<LevelType>, OutputData = StreamMessage>
        + TaskReceiver<
            SimContext = SimulationContext<LevelType>,
            InputTask = Task<LevelType::Storage>,
            LevelType = LevelType,
        > + Component<SimContext = SimulationContext<LevelType>>,
{
    type SimContext = SimulationContext<LevelType>;

    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        self.child.cycle(context, current_cycle);
        // send task to child
        if self.graph_a_tasks.current_working_target < self.graph_a_tasks.tasks.len() {
            let task_a = &mut self.graph_a_tasks.tasks[self.graph_a_tasks.current_working_target];
            if let Some(task) = task_a.front() {
                match self.child.receive_task(task, context, current_cycle) {
                    Ok(_) => {
                        debug!("task {:?} sent", task);
                        // success, remove the task from task queue
                        let task = task_a.pop_front().unwrap();
                        let task_to = task.into_task_data().unwrap().to;
                        self.recent_to = Some(task_to);
                        debug!(
                            "adding task to unfinished task list {}",
                            self.graph_a_tasks.current_working_target
                        );
                        self.unfinished_tasks.insert(task_to);
                    }
                    Err(_err_level) => {
                        // some level cannot handle the task,
                    }
                }
            } else {
                // all tasks finished
                // send the finish signal
                debug!(
                    "task for {} finished",
                    self.graph_a_tasks.current_working_target
                );
                let end_task = context.gen_end_task(self.recent_to.take().unwrap());
                self.child
                    .receive_task(&end_task, context, current_cycle)
                    .unwrap();
                self.graph_a_tasks.current_working_target += 1;
                context.current_sending_task = self.graph_a_tasks.current_working_target;
            }
        }
        // receive message from child

        let data = self.child.get_data(context, current_cycle);
        for d in data {
            let target = d.to;
            match d.message_type {
                crate::pim::task::StreamMessageType::Data(data) => {
                    tracing::trace!("received data from {:?},msg: {:?}", target, data);
                }
                crate::pim::task::StreamMessageType::End => {
                    debug!("removing from unfinished task: {:?}", target);
                    let removed = self.unfinished_tasks.remove(&target);
                    assert!(removed);
                }
            }
        }
        // decide if finished
        let is_finished = self.unfinished_tasks.is_empty()
            && self.graph_a_tasks.current_working_target == self.graph_a_tasks.tasks.len();
        context.finished = is_finished;
    }
}


#[cfg(test)]
mod tests{
    use crate::pim::level::ddr4;
    use crate::pim::level::ddr4::Mapping;
    use super::*;

    #[test]
    fn test_task_generation(){
        let total_size=ddr4::Storage{ data: [1,1,1,1,1,2,100,4] };
        let graph_a=sprs::io::read_matrix_market("mtx/test.mtx").unwrap().to_csr();
        let graph_b = graph_a.transpose_view().to_csr();
        let graph_b_mappings = Mapping::get_mapping(&total_size, &graph_b);
        let mut task_builder = TaskBuilder::default();
        let graph_a_tasks = TaskManager::<(),ddr4::Mapping>::generate_mappings_for_a(&graph_a, &graph_b_mappings,&mut task_builder);
        for task in graph_a_tasks.tasks{
            for task in task{
                println!("{:?}",task);
            }
        }

    }
}