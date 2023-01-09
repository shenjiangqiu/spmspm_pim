//! a library for creating pim simulator
// #![deny(unsafe_code)]
// #![warn(missing_docs)]
pub mod analysis;
pub mod pim;
use crate::pim::config::Config;
use clap::Parser;
use cli::{AnalyzeArgs, Cli, RunArgs};
use eyre::Result;
pub use pim::Simulator;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufWriter;
use tracing::info;
use tracing::metadata::LevelFilter;
pub mod cli;
pub mod draw;

#[allow(dead_code)]
pub fn init_logger_info() {
    init_logger(LevelFilter::INFO);
}

#[allow(dead_code)]
pub fn init_logger_debug() {
    init_logger(LevelFilter::DEBUG);
}

#[allow(dead_code)]
pub fn init_logger(filter: LevelFilter) {
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

/// the main function of the simulator
pub fn main_inner<A, T>(args: A) -> Result<()>
where
    A: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
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
                let current_time = std::time::Instant::now();
                println!("analyze with config: {:?}", config);
                let config = Config::new(config);

                let stem = config.output_path.file_stem().unwrap();
                let externsion = config.output_path.extension().unwrap();
                let new_file_name = format!(
                    "{}_split_spmm.{}",
                    stem.to_str().unwrap(),
                    externsion.to_str().unwrap()
                );
                let dir_name = config.output_path.parent().unwrap();
                let new_path = dir_name.join(new_file_name);
                info!("the result will be written to {:?}", new_path);

                let split_result = analysis::analyze_split_spmm::analyze_split_spmm(&config);
                split_result.show_results();

                serde_json::to_writer(BufWriter::new(File::create(new_path)?), &split_result)?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
            cli::AnalyzeType::Gearbox => {
                let current_time = std::time::Instant::now();
                info!("analyze with config: {:?}", config);
                let config = Config::new(config);

                let stem = config.output_path.file_stem().unwrap();
                let externsion = config.output_path.extension().unwrap();
                let new_file_name = format!(
                    "{}_gearbox.{}",
                    stem.to_str().unwrap(),
                    externsion.to_str().unwrap()
                );
                let dir_name = config.output_path.parent().unwrap();
                let new_path = dir_name.join(new_file_name);
                info!("the result will be written to {:?}", new_path);

                let gearbox_result = analysis::analyze_gearbox::analyze_gearbox(&config);
                serde_json::to_writer(BufWriter::new(File::create(new_path)?), &gearbox_result)?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
            cli::AnalyzeType::Nnz => {
                let current_time = std::time::Instant::now();
                info!("analyze with config: {:?}", config);
                let config = Config::new(config);
                let nnz_result = analysis::analyze_nnz::analyze_nnz_spmm(&config);
                nnz_result.show_results();

                serde_json::to_writer(
                    BufWriter::new(File::create(config.output_path)?),
                    &nnz_result,
                )?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
            cli::AnalyzeType::NnzNative => {
                let current_time = std::time::Instant::now();
                info!("analyze with config: {:?}", config);
                let config = Config::new(config);
                let nnz_result = analysis::analyze_nnz_native::analyze_nnz_spmm(&config);
                nnz_result.show_results();
                serde_json::to_writer(
                    BufWriter::new(File::create(config.output_path)?),
                    &nnz_result,
                )?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
            cli::AnalyzeType::GearboxParallel => {
                let current_time = std::time::Instant::now();
                info!("analyze with config: {:?}", config);
                let config = Config::new(config);

                let stem = config.output_path.file_stem().unwrap();
                let externsion = config.output_path.extension().unwrap();
                let new_file_name = format!(
                    "{}_gearbox.{}",
                    stem.to_str().unwrap(),
                    externsion.to_str().unwrap()
                );
                let dir_name = config.output_path.parent().unwrap();
                let new_path = dir_name.join(new_file_name);
                info!("the result will be written to {:?}", new_path);

                let gearbox_result = analysis::analyze_gearbox_parallel::analyze_gearbox(&config);
                serde_json::to_writer(BufWriter::new(File::create(new_path)?), &gearbox_result)?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
        },
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use sprs::{num_kinds::Pattern, CsMat};

    use crate::{main_inner, pim::config::Config, Simulator};

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

    #[test]
    fn sprs_test() {
        let matrix_a = CsMat::new(
            (3, 3),
            vec![0, 2, 4, 6],
            vec![0, 1, 0, 1, 0, 2],
            vec![Pattern; 6],
        );
        let matrix_b = CsMat::new(
            (3, 3),
            vec![0, 2, 4, 6],
            vec![0, 1, 0, 1, 0, 2],
            vec![Pattern; 6],
        );
        let matrix_c = &matrix_a * &matrix_b;
        println!("{:?}", matrix_c);
    }

    #[test]
    fn test_gearbox() -> eyre::Result<()> {
        let args = [
            "spmspm_pim",
            "analyze",
            "gearbox",
            "configs/gearbox_test.toml",
        ];
        main_inner(args)
    }
}
