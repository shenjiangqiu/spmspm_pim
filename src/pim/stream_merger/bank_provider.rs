use crate::pim::{
    bank::BankState,
    level::{LevelTrait, PathStorage},
    task::{StreamMessage, Task},
    Component, SimulationContext,
};

use super::{StreamProvider, TaskReceiver};
#[derive(Debug, Clone)]
pub struct BankProvider<LevelType: LevelTrait> {
    level: LevelType,
    bank_status: BankState,
    task_queue: Vec<Task<LevelType::Storage>>,
    ready_queue: Vec<StreamMessage>,
    max_task_queue_size: usize,
    max_provider_size: usize,
    pre_charge: u64,
    act: u64,
    /// ready cycle, row_id
    current_opening_row: Option<(u64, usize)>,
}
impl<LevelType: LevelTrait> BankProvider<LevelType> {
    pub fn new(max_provider_size: usize, max_task_queue_size: usize, pre: u64, act: u64) -> Self {
        Self {
            level: LevelType::last_level(),
            bank_status: BankState::new(),
            task_queue: Vec::new(),
            ready_queue: Vec::new(),
            max_provider_size,
            max_task_queue_size,
            current_opening_row: None,
            pre_charge: pre,
            act,
        }
    }
}

impl<LevelType: LevelTrait> StreamProvider for BankProvider<LevelType> {
    type OutputData = StreamMessage;

    type SimContext = SimulationContext<LevelType>;

    fn get_data(
        &mut self,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Vec<Self::OutputData> {
        let mut result = Vec::new();
        while let Some(data) = self.ready_queue.pop() {
            result.push(data);
        }
        result
    }
}

impl<LevelType: LevelTrait> TaskReceiver for BankProvider<LevelType> {
    type InputTask = Task<LevelType::Storage>;

    type SimContext = SimulationContext<LevelType>;

    fn receive_task(
        &mut self,
        task: Self::InputTask,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Result<(), Self::InputTask> {
        if self.task_queue.len() < self.max_task_queue_size {
            self.task_queue.push(task);
            Ok(())
        } else {
            Err(task)
        }
    }
}

impl<LevelType: LevelTrait + Copy> Component for BankProvider<LevelType> {
    type SimContext = SimulationContext<LevelType>;

    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        if let Some((ready_cycle, row_id)) = self.current_opening_row {
            if ready_cycle <= current_cycle {
                self.bank_status.open_row(row_id);
                self.current_opening_row = None;
            }
        } else if self.ready_queue.len() < self.max_provider_size {
            if let Some(task) = self.task_queue.pop() {
                // have a new task get the row id:
                let row_id = task.target_id.get_row_id();
                // check if the row is ready:
                if self.bank_status.is_row_ready(row_id) {
                    // if ready, push the task to ready queue:
                    let message = context.gen_message_from_task(&task, current_cycle);
                    self.ready_queue.push(message);
                } else {
                    // if not ready, open the row and wait
                    self.task_queue.push(task);
                    // open the row
                    let latency = if self.bank_status.is_row_opened() {
                        self.pre_charge + self.act
                    } else {
                        self.act
                    };
                    self.current_opening_row = Some((current_cycle + latency, row_id));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pim::{
        config::Config,
        level::ddr4,
        stream_merger::{StreamProvider, TaskReceiver},
        task::{PathId, Task, TaskBuilder},
        Component, SimulationContext,
    };

    use super::BankProvider;

    #[test]
    fn test_bank_provider() {
        let mut current_cycle = 0;
        let config = Config::from_ddr4(2, 2, 2);
        let mut context = SimulationContext::<ddr4::Level>::new(&config);
        let mut provider = BankProvider::<ddr4::Level>::new(10, 10, 10, 10);
        let mut task_builder = TaskBuilder::new();
        let path_storage = ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0);
        let task = task_builder.gen_task(PathId::new(path_storage), 0, 0, 1);
        provider
            .receive_task(task, &mut context, current_cycle)
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
        println!("{:?}", data);
    }
}
