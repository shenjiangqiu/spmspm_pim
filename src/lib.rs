//! a library for creating pim simulator
#![deny(unsafe_code)]
#![warn(missing_docs)]
pub mod analysis;
pub mod pim;

use clap::{Command, CommandFactory, Parser};
use clap_complete::Generator;
use cli::{AnalyzeArgs, Cli, CompArgs, RunArgs};
pub use pim::Simulator;
use tracing::info;
use tracing::metadata::LevelFilter;

use crate::pim::config::Config;
pub mod cli;
#[allow(dead_code)]
pub(crate) fn init_logger_info() {
    init_logger(LevelFilter::INFO);
}

#[allow(dead_code)]
pub(crate) fn init_logger_debug() {
    init_logger(LevelFilter::DEBUG);
}

#[allow(dead_code)]
pub(crate) fn init_logger(filter: LevelFilter) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(filter.into())
                .from_env_lossy(),
        )
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("failed to init logger: {}", e);
        });
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}
/// the main function of the simulator
pub fn main_inner() {
    let cli = Cli::parse();
    init_logger_info();

    match cli.subcmd {
        cli::Operation::Run(RunArgs { config }) => {
            println!("run with config: {:?}", config);
            let config = Config::new(config);
            info!("building simulator");
            let mut simulator = Simulator::new(&config);
            info!("start running simulator");

            simulator.run(&config);
        }
        cli::Operation::Completion(CompArgs { shell }) => {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
        }
        cli::Operation::Analyze(AnalyzeArgs { analyze, config }) => match analyze {
            cli::AnalyzeType::All => {
                println!("analyze with config: {:?}", config);
                let config = Config::new(config);
                analysis::print_all_stats(&config);
            }
            cli::AnalyzeType::Overlap => todo!(),
            cli::AnalyzeType::Sequential => todo!(),
            cli::AnalyzeType::Window => todo!(),
            cli::AnalyzeType::SplitSpmm => {
                println!("analyze with config: {:?}", config);
                let config = Config::new(config);
                let split_result = analysis::analyze_split_spmm::analyze_split_spmm(&config);
                split_result.show_results();
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{pim::config::Config, Simulator};

    #[test]
    fn it_works() {
        for i in (1..=5).rev() {
            println!("{}", i);
        }
    }

    fn pim_test_impl(mut simulator: Simulator, config: &Config) {
        simulator.run(config);
    }

    #[test]
    fn pim_test() {
        let config = Config::new("config.toml");
        match config.dram_type {
            crate::pim::config::DramType::DDR3 => todo!(),
            crate::pim::config::DramType::DDR4 => pim_test_impl(Simulator::new(&config), &config),
            crate::pim::config::DramType::LPDDR3 => todo!(),
            crate::pim::config::DramType::LPDDR4 => todo!(),
            crate::pim::config::DramType::HBM => todo!(),
            crate::pim::config::DramType::HBM2 => todo!(),
        }
    }
}
