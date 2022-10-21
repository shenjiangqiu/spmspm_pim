use std::{collections::VecDeque, fmt::Debug};

use sprs::{num_kinds::Pattern, CsMat};
use tracing::debug;

use crate::pim::{
    level::LevelTrait,
    row_buffer::BankState,
    task::{StreamMessage, Task, TaskData},
    Component, SimulationContext,
};

use super::{EmptyComponent, StreamProvider, TaskReceiver};
#[derive(Debug, Clone)]
struct WorkingTask<Storage> {
    task: TaskData<Storage>,
    current_size: usize,
    current_idx: usize,
}
#[derive(Debug, Clone)]
pub struct Provider<'a, LevelType: LevelTrait> {
    id: usize,
    level: LevelType,
    bank_status: BankState,
    task_queue: VecDeque<TaskData<LevelType::Storage>>,
    current_working_task: Option<WorkingTask<LevelType::Storage>>,
    ready_queue: VecDeque<StreamMessage>,
    max_task_queue_size: usize,
    max_provider_size: usize,
    /// the number of bytes in a row of a subarray!
    row_size: usize,
    pre_charge: u64,
    act: u64,
    /// ready cycle, row_id
    current_opening_row: Option<(u64, usize)>,
    graph_b: &'a CsMat<Pattern>,
}
impl<'a, LevelType: LevelTrait> Provider<'a, LevelType> {
    /// create a new provider
    /// ## Arguments
    /// - `id`: the id of the provider
    /// - `max_provider_size`: max number of messages can be provided in a cycle
    /// - `max_task_queue_size`: max number of tasks can be queued
    /// - `row_size`: the number of bytes in a row of a subarray!
    /// - `pre`: the number of cycles for precharge
    /// - `act`: the number of cycles for activate
    /// - `graph_b`: the graph stored in CSR format
    pub fn new(
        id: usize,
        max_provider_size: usize,
        max_task_queue_size: usize,
        row_size: usize,
        pre: u64,
        act: u64,
        graph: &'a CsMat<Pattern>,
    ) -> Self {
        Self {
            id,
            level: LevelType::last_level(),
            bank_status: BankState::new(),
            task_queue: VecDeque::new(),
            ready_queue: VecDeque::new(),
            current_working_task: None,
            max_provider_size,
            max_task_queue_size,
            current_opening_row: None,
            row_size,
            pre_charge: pre,
            act,
            graph_b: graph,
        }
    }
}

impl<'a, LevelType: LevelTrait> StreamProvider for Provider<'a, LevelType> {
    type OutputData = StreamMessage;

    type SimContext = SimulationContext<LevelType>;

    fn get_data(
        &mut self,
        _context: &mut Self::SimContext,
        _current_cycle: u64,
    ) -> Vec<Self::OutputData> {
        let mut result = Vec::new();
        while result.len() < self.max_provider_size && !self.ready_queue.is_empty() {
            result.push(self.ready_queue.pop_front().unwrap());
        }

        result
    }

    fn peek_data(
        &self,
        _context: &Self::SimContext,
        _current_cycle: u64,
    ) -> Vec<&Self::OutputData> {
        self.ready_queue
            .iter()
            .take(self.max_provider_size)
            .collect()
    }
}

impl<'a, LevelType: LevelTrait + Debug> TaskReceiver for Provider<'a, LevelType>
where
    LevelType::Storage: Clone,
{
    type LevelType = LevelType;
    type InputTask = Task<LevelType::Storage>;
    type SimContext = SimulationContext<LevelType>;

    fn receive_task(
        &mut self,
        task: &Self::InputTask,
        _context: &mut Self::SimContext,
        _current_cycle: u64,
    ) -> Result<(), Self::LevelType> {
        if let Task::TaskData(data) = task {
            if self.task_queue.len() < self.max_task_queue_size {
                debug!(
                    ?self.level,
                    self.id, "provider receive task at level:{:?} : {:?}", self.level, data
                );
                self.task_queue.push_back(data.clone());
                return Ok(());
            } else {
                return Err(self.level);
            }
        }
        // it's an end task, just ignore it
        debug!(?self.level,self.id, "receive end task at level: {:?}", self.level);
        Ok(())
    }
}

impl<'a, LevelType: LevelTrait> EmptyComponent for Provider<'a, LevelType> {
    fn is_empty(&self) -> Vec<String> {
        let mut result = Vec::new();
        if !self.current_opening_row.is_none() {
            result.push(format!(
                "provider {} is opening row {:?}",
                self.id, self.current_opening_row
            ));
        }
        if !self.current_working_task.is_none() {
            result.push(format!(
                "provider current_working_task {} is not empty",
                self.id
            ));
        }
        if !self.task_queue.is_empty() {
            result.push(format!("provider task_queue {} is not empty", self.id));
        }
        if !self.ready_queue.is_empty() {
            result.push(format!("provider ready_queue {} is not empty", self.id));
        }
        result
    }
}

impl<'a, LevelType: LevelTrait + Copy> Component for Provider<'a, LevelType> {
    type SimContext = SimulationContext<LevelType>;

    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        // a row is being opened
        if let Some((ready_cycle, row_id)) = self.current_opening_row {
            if ready_cycle <= current_cycle {
                self.bank_status.open_row(row_id);
                self.current_opening_row = None;
            }
        } else if self.ready_queue.len() < self.max_provider_size {
            // now we can try to produce a stream message

            if let Some(working_taks) = self.current_working_task.as_mut() {
                // work
                let from = working_taks.task.from;
                let row_b_idx = working_taks.current_idx;
                // have a new task get the row id:
                let row_id = working_taks.task.target_id.get_row_id();
                let row_id = row_id + working_taks.current_size / self.row_size;
                // check if the row is ready:
                if self.bank_status.is_row_ready(row_id) {
                    // if ready, push the task to ready queue:
                    let idx = self.graph_b.outer_view(from).unwrap().indices()[row_b_idx];
                    let message = context.generate_msg(working_taks.task.to, idx, current_cycle);
                    self.ready_queue.push_back(message);
                    // update the current working task:
                    working_taks.current_size += 4;
                    working_taks.current_idx += 1;
                    if working_taks.current_size >= working_taks.task.size {
                        // this task is done
                        let end_message = context.generate_end(working_taks.task.to);
                        self.current_working_task = None;
                        self.ready_queue.push_back(end_message);
                    }
                } else {
                    // if not ready, open the row and wait
                    // open the row
                    let latency = if self.bank_status.is_row_opened() {
                        self.pre_charge + self.act
                    } else {
                        self.act
                    };
                    self.current_opening_row = Some((current_cycle + latency, row_id));
                }
            } else if let Some(data) = self.task_queue.pop_front() {
                self.current_working_task = Some(WorkingTask {
                    task: data,
                    current_size: 0,
                    current_idx: 0,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::pim::{
        config::{Config, LevelConfig},
        level::ddr4,
        stream_merger::{StreamProvider, TaskReceiver},
        task::{PathId, TaskBuilder, TaskTo},
        Component, SimulationContext,
    };

    use super::Provider;

    #[test]
    fn test_bank_provider() {
        let mut current_cycle = 0;
        let config = Config::from_ddr4(
            LevelConfig {
                num: 1,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            LevelConfig {
                num: 2,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
        );
        let mut context = SimulationContext::<ddr4::Level>::new(&config);
        let graph_b = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();
        let mut provider = Provider::<ddr4::Level>::new(0, 10, 10, 10, 10, 10, &graph_b);
        let mut task_builder = TaskBuilder::default();
        let path_storage = ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0, 0);
        let task =
            task_builder.gen_task(PathId::new(path_storage), 0, TaskTo { to: 0, round: 0 }, 1);
        provider
            .receive_task(&task, &mut context, current_cycle)
            .unwrap();
        let end_task = task_builder.gen_end_task(TaskTo { to: 1, round: 0 });
        provider
            .receive_task(&end_task, &mut context, current_cycle)
            .unwrap();
        provider.cycle(&mut context, current_cycle);
        current_cycle += 1;
        let data = loop {
            let data = provider.get_data(&mut context, 0);
            if data.len() == 0 {
                provider.cycle(&mut context, current_cycle);
                current_cycle += 1;
            } else {
                break data;
            }
        };
        println!("data1: {:?}", data);
    }

    #[test]
    fn test_change_line() {
        let current_cycle = 0;
        let config = Config::from_ddr4(
            LevelConfig {
                num: 1,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            LevelConfig {
                num: 2,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
        );
        let mut context = SimulationContext::<ddr4::Level>::new(&config);
        let graph_b = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();
        let mut provider = Provider::<ddr4::Level>::new(0, 10, 10, 10, 10, 10, &graph_b);
        let mut task_builder = TaskBuilder::default();
        let path_storage = ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0, 0);
        let task =
            task_builder.gen_task(PathId::new(path_storage), 0, TaskTo { to: 0, round: 0 }, 1);
        provider
            .receive_task(&task, &mut context, current_cycle)
            .unwrap();
        let end_task = task_builder.gen_end_task(TaskTo { to: 0, round: 0 });
        provider
            .receive_task(&end_task, &mut context, current_cycle)
            .unwrap();
        let path_storage = ddr4::Storage::new(0, 0, 0, 0, 0, 0, 1, 0);

        let task =
            task_builder.gen_task(PathId::new(path_storage), 0, TaskTo { to: 1, round: 0 }, 1);
        provider
            .receive_task(&task, &mut context, current_cycle)
            .unwrap();
        let end_task = task_builder.gen_end_task(TaskTo { to: 1, round: 0 });

        provider
            .receive_task(&end_task, &mut context, current_cycle)
            .unwrap();
        provider.cycle(&mut context, current_cycle);
        let mut result = vec![];
        for i in 0..1000 {
            provider.cycle(&mut context, i);
            let data = provider.get_data(&mut context, 0);
            result.extend(data);
        }
        for r in result {
            println!("data: {:?}", r);
        }
    }
}
