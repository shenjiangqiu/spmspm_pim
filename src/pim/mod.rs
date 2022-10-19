//! the pim module

use std::{fmt::Debug, marker::PhantomData};

use sprs::{num_kinds::Pattern, CsMat};
use tracing::info;

use self::{
    config::Config,
    level::{ddr4, LevelTrait},
    stream_merger::{provider::Provider, EmptyComponent, SimpleStreamMerger},
    task::{StreamMessage, Task},
    task_manager::TaskManager,
};

pub mod config;
pub mod level;
pub mod row_buffer;
pub mod stream_merger;
pub mod task;
pub mod task_manager;

pub trait Component {
    /// the mutable context shared by all components.
    type SimContext;
    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64);
}

/// shared status for all components
pub struct SimulationContext<LevelType> {
    message_builder: task::StreamMessageBuilder,
    task_builder: task::TaskBuilder,
    level: PhantomData<LevelType>,
    finished: bool,
}

impl<LevelType> SimulationContext<LevelType> {
    /// create a new simulation context
    pub fn new(_config: &Config) -> Self {
        Self {
            level: PhantomData,
            message_builder: Default::default(),
            task_builder: Default::default(),
            finished: false,
        }
    }
    /// return if the simulation is finished
    pub fn finished(&self) -> bool {
        self.finished
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

    pub fn generate_msg(
        &mut self,
        to: usize,
        idx: usize,
        generated_cycle: u64,
    ) -> task::StreamMessage {
        self.message_builder.generate_msg(to, idx, generated_cycle)
    }

    pub fn generate_end(&mut self, to: usize) -> StreamMessage {
        self.message_builder.generate_end_msg(to)
    }

    pub fn gen_end_task<Storage>(&mut self, to: usize) -> Task<Storage> {
        self.task_builder.gen_end_task(to)
    }
}

/// the simulator struct which contains all components.
pub struct Simulator {
    cycle: u64,
}
impl Simulator {
    /// create a new simulator with config
    pub fn new(_config: &Config) -> Self {
        Self { cycle: 0 }
    }

    /// run the simulator and print the statistics
    pub fn run(&mut self, config: &Config) {
        let mut context = SimulationContext::<ddr4::Level>::new(config);
        let graph_a = sprs::io::read_matrix_market(&config.graph_path)
            .unwrap()
            .to_csr();
        let graph_b = graph_a.transpose_view().to_csr();
        match config.dram_type {
            config::DramType::DDR3 => todo!(),
            config::DramType::DDR4 => {
                self.run_inner(
                    config,
                    &mut context,
                    Self::build_merger_ddr4(config, &graph_a, &graph_b),
                );
            }
            config::DramType::LPDDR3 => todo!(),
            config::DramType::LPDDR4 => todo!(),
            config::DramType::HBM => todo!(),
            config::DramType::HBM2 => todo!(),
        }
    }
    fn run_inner<LevelType: LevelTrait>(
        &mut self,
        _config: &Config,
        context: &mut SimulationContext<LevelType>,
        mut merger: impl Component<SimContext = SimulationContext<LevelType>> + EmptyComponent + Debug,
    ) {
        while !context.finished() {
            merger.cycle(context, self.cycle);
            self.cycle += 1;
        }
        // run extra 10 cycle to perform status update
        for _ in 0..10 {
            merger.cycle(context, self.cycle);
            self.cycle += 1;
        }

        // finished,
        // make sure all tasks are finished
        match merger.is_empty() {
            Ok(_) => {
                info!("simulation finished");
            }
            Err(err_msg) => {
                tracing::error!("simulation failed: {}", err_msg);
            }
        }
        println!("simulator: {:?}", merger);
    }

    fn build_merger_ddr4<'a>(
        config: &Config,
        graph_a: &CsMat<Pattern>,
        graph_b: &'a CsMat<Pattern>,
    ) -> impl Component<SimContext = SimulationContext<ddr4::Level>> + EmptyComponent + Debug + 'a
    {
        let subarray_provider: Vec<_> = (0..config.subarrays)
            .map(|id| Provider::new(id, 1, 1, 1, 1, 1, graph_b))
            .collect();
        let bank_merger: Vec<_> = (0..config.banks)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    &config,
                    subarray_provider.clone(),
                    ddr4::Level::Bank,
                    config.pe_num,
                    1,
                    1,
                    1,
                )
            })
            .collect();
        let bg_merger: Vec<_> = (0..config.bank_groups)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    &config,
                    bank_merger.clone(),
                    ddr4::Level::BankGroup,
                    config.pe_num,
                    1,
                    1,
                    1,
                )
            })
            .collect();

        let chip_merger: Vec<_> = (0..config.chips)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    &config,
                    bg_merger.clone(),
                    ddr4::Level::Chip,
                    config.pe_num,
                    1,
                    1,
                    1,
                )
            })
            .collect();
        let rank_merger = (0..config.ranks)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    &config,
                    chip_merger.clone(),
                    ddr4::Level::Rank,
                    config.pe_num,
                    1,
                    1,
                    1,
                )
            })
            .collect();
        let channel_merger = SimpleStreamMerger::new(
            0,
            &config,
            rank_merger,
            ddr4::Level::Channel,
            config.pe_num,
            1,
            1,
            1,
        );

        let total_size = ddr4::Storage::new(
            config.channels,
            config.ranks,
            config.chips,
            config.bank_groups,
            config.banks,
            config.subarrays,
            config.rows,
            config.row_size,
        );
        TaskManager::new(channel_merger, graph_a, graph_b, &total_size)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::init_logger_debug;
    use tracing::debug;

    #[test]
    fn test_real_graph() {
        init_logger_debug();
        debug!("test real graph");
        let config = Config::from_ddr4(1, 2, 10);
        let mut simulator = Simulator::new(&config);
        simulator.run(&config);
    }
}
