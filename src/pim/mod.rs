//! the pim module

use std::{fmt::Debug, fs, marker::PhantomData, process::exit};

use serde::Serialize;
use sprs::{num_kinds::Pattern, CsMat};
use tracing::info;

use self::{
    config::Config,
    level::{ddr4, LevelTrait},
    stream_merger::{provider::Provider, EmptyComponent, SimpleStreamMerger},
    task::{StreamMessage, TaskEndData, TaskTo},
    task_manager::TaskManager,
};

pub mod config;
pub mod configv2;
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
#[derive(Debug)]
pub struct SimulationContext<LevelType> {
    message_builder: task::StreamMessageBuilder,
    task_builder: task::TaskBuilder,
    level: PhantomData<LevelType>,
    finished: bool,
    stats: Statistics,
    current_sending_task: usize,
    total_tasks: usize,
}
#[derive(Serialize, Debug, Default)]
struct Statistics {
    cycle: u64,
}

impl<LevelType: LevelTrait> SimulationContext<LevelType> {
    /// create a new simulation context
    pub fn new(_config: &Config) -> Self {
        Self {
            level: PhantomData,
            message_builder: Default::default(),
            task_builder: Default::default(),
            finished: false,
            current_sending_task: 0,
            stats: Default::default(),
            total_tasks: 0,
        }
    }
    /// return if the simulation is finished
    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn gen_task(
        &mut self,
        target_id: task::PathId<LevelType>,
        from: usize,
        to: TaskTo,
        size: usize,
    ) -> task::TaskData<LevelType> {
        self.task_builder.gen_task(target_id, from, to, size)
    }

    pub fn generate_msg(&mut self, to: TaskTo, idx: usize, generated_cycle: u64) -> StreamMessage {
        self.message_builder.generate_msg(to, idx, generated_cycle)
    }

    pub fn generate_end(&mut self, to: TaskTo) -> StreamMessage {
        self.message_builder.generate_end_msg(to)
    }

    pub fn gen_end_task(&mut self, to: TaskTo) -> TaskEndData {
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
        for graph in &config.graph_path {
            info!("run config: {:?}", config);
            let mut context = SimulationContext::<ddr4::Level>::new(config);
            info!("generating graph A: {:?}", config.graph_path);
            let graph_a = sprs::io::read_matrix_market(graph).unwrap().to_csr();
            info!("generating graph B: {:?}", config.graph_path);

            let graph_b = graph_a.transpose_view().to_csr();
            match config.dram_type {
                config::DramType::DDR3 => todo!(),
                config::DramType::DDR4 => {
                    info!("using DDR4");
                    let merger = Self::build_merger_ddr4(config, &graph_a, &graph_b, &mut context);
                    self.run_inner(config, &mut context, merger);
                }
                config::DramType::LPDDR3 => todo!(),
                config::DramType::LPDDR4 => todo!(),
                config::DramType::HBM => todo!(),
                config::DramType::HBM2 => todo!(),
            }
        }
    }
    fn run_inner<LevelType: LevelTrait + Debug>(
        &mut self,
        config: &Config,
        context: &mut SimulationContext<LevelType>,
        mut merger: impl Component<SimContext = SimulationContext<LevelType>> + EmptyComponent + Debug,
    ) {
        info!("start simulation");
        let start_time = std::time::Instant::now();
        let mut last_record_time = std::time::Instant::now();
        let mut last_target = 0;

        while !context.finished() {
            merger.cycle(context, self.cycle);
            self.cycle += 1;
            if self.cycle % 100000 == 0 {
                info!("-------------------");

                let total_time = start_time.elapsed();
                let last_time = last_record_time.elapsed();
                last_record_time = std::time::Instant::now();
                let cycles_per_second = 100000.0 / last_time.as_secs_f64();
                let cycles_per_second_total = self.cycle as f64 / total_time.as_secs_f64();
                info!(
                    "cycle: {}, this_round_cycle_per_sec: {}, total_cycle_per_sec: {}",
                    self.cycle, cycles_per_second, cycles_per_second_total
                );
                let current_target = context.current_sending_task;
                let tasks_this_round = current_target - last_target;
                last_target = current_target;

                let this_round_task_per_sec = tasks_this_round as f64 / last_time.as_secs_f64();
                let total_task_per_sec = current_target as f64 / total_time.as_secs_f64();
                info!(
                    "tasks: {}/{} this_round_task_per_sec: {}, total_task_per_sec: {}",
                    current_target,
                    context.total_tasks,
                    this_round_task_per_sec,
                    total_task_per_sec
                );
                let guessed_finished_time = total_time
                    .mul_f64((context.total_tasks as f64) / (current_target as f64))
                    - total_time;
                info!(
                    "guessed finished time: {}",
                    humantime::format_duration(guessed_finished_time)
                );

                println!("current runing status: {:?}", context);

                if self.cycle == 2000000 {
                    exit(1);
                }
            }
        }
        info!("simulation finished");
        // run extra 10 cycle to perform status update
        for _ in 0..10 {
            merger.cycle(context, self.cycle);
            self.cycle += 1;
        }

        // finished,
        // make sure all tasks are finished
        let result = merger.is_empty();
        if result.is_empty() {
            info!("simulation finished");
            info!("stats: {:?}", context.stats);
            info!("context has been saved to {:?}", config.output_path);
            if let Some(ouput_dir) = config.output_path.parent() {
                // create the output dir if not exists
                fs::create_dir_all(ouput_dir).unwrap();
            }
            serde_json::to_writer_pretty(
                fs::File::create(&config.output_path).unwrap(),
                &context.stats,
            )
            .unwrap();
            context.stats.cycle = self.cycle;
            info!("total cycle: {}", self.cycle);
        } else {
            tracing::error!("simulation failed: {:?}", result);
            tracing::error!("simulator: {:?}", merger);
        }
    }

    fn build_merger_ddr4<'a>(
        config: &Config,
        graph_a: &CsMat<Pattern>,
        graph_b: &'a CsMat<Pattern>,
        sim_context: &mut SimulationContext<ddr4::Level>,
    ) -> impl Component<SimContext = SimulationContext<ddr4::Level>> + EmptyComponent + Debug + 'a
    {
        let subarray_provider: Vec<_> = (0..config.subarrays)
            .map(|id| Provider::new(id, 1, 1, 1, 1, 1, graph_b))
            .collect();
        let bank_merger: Vec<_> = (0..config.banks.num)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    config,
                    subarray_provider.clone(),
                    ddr4::Level::Bank,
                    config.banks.merger_num,
                    config.banks.max_msg_in,
                    config.banks.max_msg_generated,
                    config.banks.max_msg_out,
                )
            })
            .collect();
        let bg_merger: Vec<_> = (0..config.bank_groups.num)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    config,
                    bank_merger.clone(),
                    ddr4::Level::BankGroup,
                    config.bank_groups.merger_num,
                    config.bank_groups.max_msg_in,
                    config.bank_groups.max_msg_generated,
                    config.bank_groups.max_msg_out,
                )
            })
            .collect();

        let chip_merger: Vec<_> = (0..config.chips.num)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    config,
                    bg_merger.clone(),
                    ddr4::Level::Chip,
                    config.chips.merger_num,
                    config.chips.max_msg_in,
                    config.chips.max_msg_generated,
                    config.chips.max_msg_out,
                )
            })
            .collect();
        let rank_merger = (0..config.ranks.num)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    config,
                    chip_merger.clone(),
                    ddr4::Level::Rank,
                    config.ranks.merger_num,
                    config.ranks.max_msg_in,
                    config.ranks.max_msg_generated,
                    config.ranks.max_msg_out,
                )
            })
            .collect();
        let channel_merger = SimpleStreamMerger::new(
            0,
            config,
            rank_merger,
            ddr4::Level::Channel,
            config.channels.merger_num,
            config.channels.max_msg_in,
            config.channels.max_msg_generated,
            config.channels.max_msg_out,
        );

        let total_size = ddr4::Storage::new(
            config.channels.num,
            config.ranks.num,
            config.chips.num,
            config.bank_groups.num,
            config.banks.num,
            config.subarrays,
            config.rows,
            config.columns,
        );
        TaskManager::new(channel_merger, graph_a, graph_b, &total_size, sim_context)
    }
}

// #[cfg(test)]
// mod tests {

//     use super::*;
//     use crate::{init_logger_debug, pim::config::LevelConfig};
//     use tracing::debug;

//     #[test]
//     fn test_real_graph() {
//         init_logger_debug();
//         debug!("test real graph");
//         let config = Config::from_ddr4_3200(
//             LevelConfig {
//                 num: 1,
//                 merger_num: 10,
//                 max_msg_in: 2,
//                 max_msg_out: 2,
//                 max_msg_generated: 2,
//             },
//             LevelConfig {
//                 num: 2,
//                 merger_num: 10,
//                 max_msg_in: 2,
//                 max_msg_out: 2,
//                 max_msg_generated: 2,
//             },
//         );
//         let mut simulator = Simulator::new(&config);
//         simulator.run(&config);
//     }
// }
