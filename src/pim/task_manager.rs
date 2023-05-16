use std::{
    collections::{BTreeSet, VecDeque},
    fmt::Debug,
    ops::RangeFull,
};

use sprs::{num_kinds::Pattern, CsMat};
use tracing::debug;

use super::{
    level::{GraphBRow, LevelTrait},
    stream_merger::{EmptyComponent, StreamProvider, TaskReceiver},
    task::{PathId, StreamMessage, Task, TaskData, TaskTo},
    Component, SimulationContext,
};

/// track the finish of tasks
/// - will init the final result
/// - when finished one taks, will update the final result
/// - when all tasks finished, will return the final result
#[derive(Debug)]
pub struct TaskManager<Child, LevelType: LevelTrait> {
    child: Child,
    graph_a_tasks: GraphATasks<LevelType>,
    unfinished_tasks: BTreeSet<TaskTo>,
    recent_to: Option<TaskTo>,
}

impl<Child: EmptyComponent, LevelType: LevelTrait> EmptyComponent
    for TaskManager<Child, LevelType>
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

/// the tasks of a round
#[derive(Debug)]
pub struct RoundTasks<LevelType: LevelTrait> {
    pub round_id: usize,
    pub tasks: VecDeque<TaskData<LevelType>>,
}

/// the tasks for graph a row
#[derive(Debug)]
pub struct GraphARowTasks<LevelType: LevelTrait> {
    pub row_id: usize,
    pub tasks: VecDeque<RoundTasks<LevelType>>,
}

/// all tests for a graph_a
#[derive(Debug)]
pub struct GraphATasks<LevelType: LevelTrait> {
    pub current_working_target: usize,

    /// for each target, the froms
    pub tasks: VecDeque<GraphARowTasks<LevelType>>,
}

pub struct Iter<'a, LevelType: LevelTrait> {
    tasks: &'a GraphATasks<LevelType>,
    current_row: usize,
    current_round: usize,
    current_task: usize,
}
impl<'a, LevelType: LevelTrait> Iterator for Iter<'a, LevelType> {
    type Item = &'a TaskData<LevelType>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row >= self.tasks.tasks.len() {
            return None;
        }
        let row = &self.tasks.tasks[self.current_row];
        if self.current_round >= row.tasks.len() {
            self.current_row += 1;
            self.current_round = 0;
            self.current_task = 0;
            return self.next();
        }
        let round = &row.tasks[self.current_round];
        if self.current_task >= round.tasks.len() {
            self.current_round += 1;
            self.current_task = 0;
            return self.next();
        }
        let task = &round.tasks[self.current_task];
        self.current_task += 1;
        Some(task)
    }
}

pub struct IntoIter<LevelType: LevelTrait> {
    tasks: GraphATasks<LevelType>,
}

pub struct IntoIterRound<LevelType: LevelTrait> {
    tasks: GraphATasks<LevelType>,
}

impl<LevelType: LevelTrait> Iterator for IntoIterRound<LevelType> {
    type Item = RoundTasks<LevelType>;

    fn next(&mut self) -> Option<Self::Item> {
        // skip the empty rows
        while !self.tasks.tasks.is_empty() && self.tasks.tasks[0].tasks.is_empty() {
            self.tasks.tasks.pop_front();
        }

        if let Some(row) = self.tasks.tasks.front_mut() {
            if let Some(round) = row.tasks.pop_front() {
                Some(round)
            } else {
                self.tasks.tasks.pop_front();
                self.next()
            }
        } else {
            None
        }
    }
}

impl<LevelType: LevelTrait> Iterator for IntoIter<LevelType> {
    type Item = TaskData<LevelType>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(row) = self.tasks.tasks.front_mut() {
            if let Some(round) = row.tasks.front_mut() {
                if let Some(task) = round.tasks.pop_front() {
                    Some(task)
                } else {
                    row.tasks.pop_front();
                    self.next()
                }
            } else {
                self.tasks.tasks.pop_front();
                self.next()
            }
        } else {
            None
        }
    }
}

impl<LevelType: LevelTrait> IntoIterator for GraphATasks<LevelType> {
    type Item = TaskData<LevelType>;
    type IntoIter = IntoIter<LevelType>;
    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}
impl<'a, LevelType: LevelTrait> IntoIterator for &'a GraphATasks<LevelType> {
    type Item = &'a TaskData<LevelType>;

    type IntoIter = Iter<'a, LevelType>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<LevelType: LevelTrait> GraphATasks<LevelType> {
    pub fn iter(&self) -> Iter<'_, LevelType> {
        Iter {
            tasks: self,
            current_row: 0,
            current_round: 0,
            current_task: 0,
        }
    }
    pub fn into_round_iter(self) -> IntoIterRound<LevelType> {
        IntoIterRound { tasks: self }
    }

    /// from graph a genrate a list of froms
    pub fn generate_mappings_for_a(
        graph_a: &CsMat<Pattern>,
        graph_b_mappings: &LevelType::Mapping,
        context: &mut SimulationContext<LevelType>,
    ) -> GraphATasks<LevelType>
    where
        LevelType::Storage: Ord,
    {
        let mut tasks: VecDeque<GraphARowTasks<LevelType>> = VecDeque::new();
        for (to, row) in graph_a.outer_iterator().enumerate() {
            let patches: Vec<(usize, &GraphBRow<LevelType::Storage>)> = row
                .indices()
                .iter()
                .map(|row_b_idx| {
                    let graph_b_row_detail =
                        LevelType::get_row_detail(graph_b_mappings, *row_b_idx);
                    (*row_b_idx, graph_b_row_detail)
                })
                .collect();

            let mut current_round = patches;
            let mut next_round = Vec::new();
            let mut current_round_num = 0;
            let mut this_target_tasks = GraphARowTasks {
                row_id: to,
                tasks: VecDeque::new(),
            };
            while !current_round.is_empty() {
                let mut uniq_set = BTreeSet::new();
                let mut this_round_tasks = RoundTasks {
                    round_id: current_round_num,
                    tasks: VecDeque::new(),
                };
                for (from, row_detail) in current_round.drain(RangeFull) {
                    if uniq_set
                        .insert(LevelType::last_level().get_sub_path_to_level(&row_detail.path))
                    {
                        // yes , it's uniq
                        //generate the task
                        let task = context.gen_task(
                            PathId::new(row_detail.path.clone()),
                            from,
                            TaskTo {
                                to,
                                round: current_round_num,
                            },
                            row_detail.size,
                        );
                        this_round_tasks.tasks.push_back(task);
                    } else {
                        // no, it's not uniq,
                        next_round.push((from, row_detail));
                    }
                }
                // let end_task = context.gen_end_task(TaskTo {
                //     to,
                //     round: current_round_num,
                // });
                // this_round_tasks.tasks.push_back(end_task);
                this_target_tasks.tasks.push_back(this_round_tasks);
                current_round = next_round.drain(RangeFull).collect();
                current_round_num += 1;
            }
            tasks.push_back(this_target_tasks);
        }
        context.total_tasks = tasks.len();
        GraphATasks {
            current_working_target: 0,
            tasks,
        }
    }
}

impl<Child, LevelType: LevelTrait> TaskManager<Child, LevelType> {
    pub fn new(
        child: Child,
        graph_a: &CsMat<Pattern>,
        graph_b: &CsMat<Pattern>,
        total_size: &LevelType::Storage,
        context: &mut SimulationContext<LevelType>,
    ) -> Self
    where
        LevelType::Storage: Ord,
    {
        let graph_b_mappings = LevelType::get_mapping(total_size, graph_b);
        let graph_a_tasks =
            GraphATasks::generate_mappings_for_a(graph_a, &graph_b_mappings, context);

        Self {
            child,
            graph_a_tasks,
            unfinished_tasks: BTreeSet::new(),
            recent_to: None,
        }
    }
}

impl<Child, LevelType: LevelTrait + Debug> Component for TaskManager<Child, LevelType>
where
    LevelType::Storage: Clone + Debug,
    Child: StreamProvider<SimContext = SimulationContext<LevelType>, OutputData = StreamMessage>
        + TaskReceiver<
            SimContext = SimulationContext<LevelType>,
            InputTask = Task<LevelType>,
            LevelType = LevelType,
        > + Component<SimContext = SimulationContext<LevelType>>,
{
    type SimContext = SimulationContext<LevelType>;

    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        self.child.cycle(context, current_cycle);
        // send task to child
        if self.graph_a_tasks.current_working_target < self.graph_a_tasks.tasks.len() {
            let row_task = &mut self.graph_a_tasks.tasks[self.graph_a_tasks.current_working_target];
            if let Some(round_task) = row_task.tasks.front_mut() {
                if let Some(task) = round_task.tasks.pop_front() {
                    match self
                        .child
                        .receive_task(Task::TaskData(task), context, current_cycle)
                    {
                        Ok(_) => {
                            // debug!("task {:?} sent", task);
                            // success, remove the task from task queue
                            round_task.tasks.pop_front().unwrap();
                            let task_to = TaskTo {
                                to: row_task.row_id,
                                round: round_task.round_id,
                            };
                            self.recent_to = Some(task_to);
                            debug!(
                                "adding task to unfinished task list {}",
                                self.graph_a_tasks.current_working_target
                            );
                            self.unfinished_tasks.insert(task_to);
                        }
                        Err((_err_level, task)) => {
                            // some level cannot handle the task,
                            // put it back to the queue
                            round_task.tasks.push_front(task.into_task_data().unwrap());
                        }
                    }
                } else {
                    // push end task
                    let task_to = TaskTo {
                        to: row_task.row_id,
                        round: round_task.round_id,
                    };
                    let end_task = context.gen_end_task(task_to);
                    self.child
                        .receive_task(Task::End(end_task), context, current_cycle)
                        .unwrap();
                    // next round
                    row_task.tasks.pop_front();
                }
            } else {
                // all tasks of this row finished
                // send the finish signal
                // next row
                debug!(
                    "task for {} finished",
                    self.graph_a_tasks.current_working_target
                );

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
mod tests {
    use super::*;
    use crate::pim::{
        config::{Config, LevelConfig},
        level::ddr4,
    };

    #[test]
    fn test_task_generation() {
        let total_size = ddr4::Storage {
            data: [1, 1, 1, 1, 1, 2, 100, 4],
        };
        let graph_a = sprs::io::read_matrix_market("test_mtx/test.mtx")
            .unwrap()
            .to_csr();
        let graph_b = graph_a.transpose_view().to_csr();
        let graph_b_mappings = ddr4::Level::get_mapping(&total_size, &graph_b);
        let mut context = SimulationContext::new(&Config::from_ddr4_3200(
            LevelConfig::default(),
            LevelConfig::default(),
        ));
        let graph_a_tasks: GraphATasks<ddr4::Level> =
            GraphATasks::generate_mappings_for_a(&graph_a, &graph_b_mappings, &mut context);
        for task in graph_a_tasks.tasks {
            for task in task.tasks {
                println!("{:?}", task);
            }
        }
    }
    #[test]
    fn test_iter() {
        // let a = vec![1, 2, 3];
        // let mut iter = a.iter();
        // let mut iter3 = (&a).into_iter();
    }
}
