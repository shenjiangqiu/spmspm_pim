use std::marker::PhantomData;

use self::{
    config::Config,
    level::{ddr4, LevelTrait},
    stream_merger::{bank_provider::BankProvider, SimpleStreamMerger},
};
mod bank;
pub mod config;
pub mod controller;
pub mod level;
pub mod merger;
pub mod stream_merger;
pub mod task;
pub trait Component {
    /// the mutable context shared by all components.
    type SimContext;
    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64);
}
struct DramPeBuffer {
    buffer: Vec<SinglePeBuffer>,
}
struct SinglePeBuffer {
    buffer: Option<task::StreamMessage>,
}
struct LevelBuffer<Level: LevelTrait> {
    level: Level,
    buffer: Vec<DramPeBuffer>,
}

/// shared status for all components
pub struct SimulationContext<LevelType> {
    message_builder: task::StreamMessageBuilder,
    task_builder: task::TaskBuilder,
    level: PhantomData<LevelType>,
}

impl<LevelType: LevelTrait> SimulationContext<LevelType> {
    /// create a new simulation context
    pub fn new(config: &Config) -> Self {
        Self {
            level: PhantomData,
            message_builder: Default::default(),
            task_builder: Default::default(),
        }
    }
    /// return if the simulation is finished
    pub fn finished(&self) -> bool {
        todo!()
    }

    pub fn gen_task<Storage>(
        &mut self,
        target_id: task::PathId<Storage>,
        from: usize,
        to: usize,
        size: usize,
    ) -> task::Task<Storage> {
        self.task_builder.gen_task(target_id, from, to, size)
    }

    pub fn gen_message_from_task<Storage>(
        &mut self,
        task: &task::Task<Storage>,
        generated_cycle: u64,
    ) -> task::StreamMessage {
        self.message_builder
            .gen_message_from_task(task, generated_cycle)
    }
}

/// the simulator struct which contains all components.
pub struct Simulator<T: LevelTrait> {
    cycle: u64,
    context: SimulationContext<T>,
}
impl<T: LevelTrait> Simulator<T> {
    /// create a new simulator with config
    pub fn new(config: &Config) -> Self {
        Self {
            cycle: 0,
            context: SimulationContext::new(&config),
        }
    }

    /// run the simulator and print the statistics
    pub fn run(&mut self, config: &Config) {
        // let mut stream_collector = BankProvider::new(
        //     config.bank_provider_size,
        //     config.bank_task_queue_size,
        //     config.precharge_cycle,
        //     config.activate_cycle,
        // );

        // let mut stream_merger =
        //     SimpleStreamMerger::new(&config, vec![stream_collector], ddr4::Level::Channel);
        // loop {
        //     // self.cycle += 1;
        //     // stream_collector.cycle(&mut self.context, self.cycle);
        //     // stream_merger.cycle(&mut self.context, self.cycle);
        //     // if self.context.finished() {
        //     //     break;
        //     // }
        // }
    }

    /// return the context
    pub fn context(&self) -> &SimulationContext<T> {
        &self.context
    }
}
