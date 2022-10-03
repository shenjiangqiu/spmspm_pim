use crate::pim::{config::Config, level::LevelTrait, task::Task, Component, SimulationContext};

use super::{StreamProvider, TaskReceiver};
#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub current_target: usize,
}

#[derive(Debug, Clone, Default)]
pub enum MergerStatus {
    #[default]
    Idle,
    WaitingTask,
    WaitingData,
}

#[derive(Default, Clone)]
pub struct SingleMergerStatus {
    current_status: MergerStatus,
}

impl SingleMergerStatus {
    pub fn new() -> Self {
        Self {
            current_status: MergerStatus::Idle,
        }
    }

    pub fn receive<LevelType>(&mut self, task: &Task<LevelType>) -> bool {}
}

pub struct SimpleStreamMerger<LevelType, Child> {
    current_level: LevelType,
    children: Vec<Child>,
    mergers: Vec<SingleMergerStatus>,
}

impl<LevelType, Child> SimpleStreamMerger<LevelType, Child> {
    pub fn new(config: &Config, children: Vec<Child>, level: LevelType) -> Self {
        Self {
            current_level: level,
            children,
            mergers: vec![SingleMergerStatus::default(); config.pe_num],
        }
    }
}

impl<LevelType, Child> TaskReceiver for SimpleStreamMerger<LevelType, Child>
where
    Child: TaskReceiver<
        InputTask = Task<LevelType::Storage>,
        SimContext = SimulationContext<LevelType>,
    >,
    LevelType: LevelTrait,
{
    type InputTask = Task<LevelType::Storage>;

    type SimContext = SimulationContext<LevelType>;

    fn receive_task(
        &mut self,
        task: Self::InputTask,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Result<(), Self::InputTask> {
        // send task to lower pe and record the task
        let child_level = self
            .current_level
            .get_child_level()
            .expect("no child level");
        let child_id = task.target_id.get_level_id(&child_level);
        self.children[child_id].receive_task(task, context, current_cycle)?;
        // record this task

        Ok(())
    }
}

impl<LevelType, Child> StreamProvider for SimpleStreamMerger<LevelType, Child>
where
    Child: StreamProvider,
    LevelType: LevelTrait,
{
    type OutputData = Child::OutputData;

    type SimContext = SimulationContext<LevelType>;

    fn get_data(
        &mut self,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Vec<Self::OutputData> {
        todo!()
    }
}

impl<LevelType, Child> Component for SimpleStreamMerger<LevelType, Child>
where
    LevelType: LevelTrait,
{
    type SimContext = SimulationContext<LevelType>;

    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use std::vec;

    use crate::pim::{level::ddr4, stream_merger::bank_provider::BankProvider, task::PathId};

    use super::*;
    #[test]
    fn test_simple_stream_merger() {
        let config = Config::from_ddr4(2, 2, 2);
        let children = vec![BankProvider::<ddr4::Level>::new(2, 2, 2, 2); 2];
        let mut context = SimulationContext::new(&config);
        let mut merger = SimpleStreamMerger::new(&config, children, ddr4::Level::BankGroup);
        let path = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0));
        let task = context.gen_task(path, 0, 0, 0);
        let mut current_cycle = 0;
        merger
            .receive_task(task, &mut context, current_cycle)
            .unwrap();
        let message = loop {
            let message = merger.get_data(&mut context, current_cycle);
            if !message.is_empty() {
                break message;
            }
            merger.cycle(&mut context, current_cycle);
            current_cycle += 1;
        };
        println!("{:?}", message);
    }
}
