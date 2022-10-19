use std::{
    collections::{BTreeSet, VecDeque},
    fmt::Debug,
};

use sprs::{num_kinds::Pattern, CsMat};
use tracing::debug;

use super::{
    level::{self, LevelTrait, MatrixBMapping},
    stream_merger::{EmptyComponent, StreamProvider, TaskReceiver},
    task::{PathId, StreamMessage, Task},
    Component, SimulationContext,
};

/// track the finish of tasks
/// - will init the final result
/// - when finished one taks, will update the final result
/// - when all tasks finished, will return the final result
#[derive(Debug)]
pub struct TaskManager<Child, Mapping> {
    child: Child,
    graph_a_tasks: GraphATasks,
    graph_b_mappings: Mapping,
    unfinished_tasks: BTreeSet<usize>,
}

impl<Child: EmptyComponent, Mapping> EmptyComponent for TaskManager<Child, Mapping> {
    fn is_empty(&self) -> Result<(), String> {
        if !self.unfinished_tasks.is_empty() {
            return Err("unfinished tasks".to_string());
        }
        self.child.is_empty()
    }
}
#[derive(Debug)]
pub struct GraphATasks {
    current_working_target: usize,

    /// for each target, the froms
    tasks: VecDeque<VecDeque<usize>>,
}

impl<Child, Mapping> TaskManager<Child, Mapping>
where
    Mapping: level::MatrixBMapping,
{
    pub fn new(
        child: Child,
        graph_a: &CsMat<Pattern>,
        graph_b: &CsMat<Pattern>,
        total_size: &Mapping::Storage,
    ) -> Self {
        let graph_b_mappings = Mapping::get_mapping(total_size, graph_b);
        let graph_a_tasks = Self::generate_mappings_for_a(graph_a);

        Self {
            child,
            graph_b_mappings,
            graph_a_tasks,
            unfinished_tasks: BTreeSet::new(),
        }
    }

    pub fn generate_mappings_for_a(graph_a: &CsMat<Pattern>) -> GraphATasks {
        let mut tasks = VecDeque::new();
        for row in graph_a.outer_iterator() {
            let task_b = row.iter().map(|(col, _data)| col).collect();

            tasks.push_back(task_b);
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
            if let Some(row_b_id) = task_a.front() {
                // build the task
                let row_b_info = self.graph_b_mappings.get_row_detail(*row_b_id);
                let path_id = row_b_info.path.clone();
                let path_id = PathId::new(path_id);
                let task = context.gen_task(
                    path_id,
                    *row_b_id,
                    self.graph_a_tasks.current_working_target,
                    row_b_info.size,
                );
                match self.child.receive_task(&task, context, current_cycle) {
                    Ok(_) => {
                        debug!("task {:?} sent", task);
                        // success, remove the task from task queue
                        task_a.pop_front();
                        debug!(
                            "adding task to unfinished task list {}",
                            self.graph_a_tasks.current_working_target
                        );
                        self.unfinished_tasks
                            .insert(self.graph_a_tasks.current_working_target);
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
                let end_task = context.gen_end_task(self.graph_a_tasks.current_working_target);
                self.child
                    .receive_task(&end_task, context, current_cycle)
                    .unwrap();
                self.graph_a_tasks.current_working_target += 1;
            }
        }
        // receive message from child

        let data = self.child.get_data(context, current_cycle);
        for d in data {
            let target = d.to;
            match d.message_type {
                crate::pim::task::StreamMessageType::Data(data) => {
                    tracing::trace!("received data from {},msg: {:?}", target, data);
                }
                crate::pim::task::StreamMessageType::End => {
                    debug!("removing from unfinished task: {}", target);
                    let removed = self.unfinished_tasks.remove(&target);
                    assert!(removed);
                }
            }
        }
        // decide if finished
        let is_finihsed = self.unfinished_tasks.is_empty()
            && self.graph_a_tasks.current_working_target == self.graph_a_tasks.tasks.len();
        context.finished = is_finihsed;
    }
}
