//! a library for creating pim simulator
// #![deny(unsafe_code)]
// #![warn(missing_docs)]
pub mod analysis;
pub mod pim;
use crate::pim::config::Config;
use clap::Parser;
use cli::{AnalyzeArgs, Cli, RunArgs};
use eyre::Result;
use once_cell::sync::Lazy;
pub use pim::Simulator;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufWriter};
use std::sync::{Condvar, Mutex};
use sysinfo::SystemExt;
use tracing::info;
use tracing::metadata::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;
pub mod cli;
pub mod draw;
pub static mut CTRL_C: bool = false;
#[allow(dead_code)]
pub fn init_logger_info() {
    init_logger(LevelFilter::INFO, io::stderr);
}

#[allow(dead_code)]
pub fn init_logger_debug() {
    init_logger(LevelFilter::DEBUG, io::stderr);
}

#[allow(dead_code)]
pub fn init_logger(
    filter: LevelFilter,
    writter: impl for<'writer> MakeWriter<'writer> + 'static + Send + Sync,
) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(filter.into())
                .from_env_lossy(),
        )
        .with_writer(writter)
        .with_ansi(false)
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("failed to init logger: {}", e);
        });
}
static CURRENT_MEMORY_USAGE: Mutex<usize> = Mutex::new(0);
static MEMORY_CONDVAR: Condvar = Condvar::new();
fn get_memory_free() -> usize {
    let mut sysinfo = sysinfo::System::new_all();
    sysinfo.refresh_memory();
    sysinfo.available_memory() as usize
}
/// parse the memory limit from the environment variable MEMORY_LIMIT, if error, return the current memory available
fn parse_memory_limit() -> usize {
    let limit = match env::var("MEMORY_LIMIT") {
        Ok(limmit) => parse_size::parse_size(&limmit)
            .map(|x| x as usize)
            .unwrap_or_else(|e| {
                eprintln!("failed to parse memory limit: {}", e);
                get_memory_free()
            }),
        Err(_) => get_memory_free(),
    };
    info!("memory limit: {} bytes", limit);
    limit
}

static TOTAL_MEMORY: Lazy<usize> = Lazy::new(|| parse_memory_limit());
pub struct MemoryGuard(usize);
#[must_use]
pub fn acquire_memory(size: usize) -> MemoryGuard {
    info!("acquire memory: {} bytes", size);
    if size > *TOTAL_MEMORY {
        panic!("memory limit exceeded");
    }
    let mut memory = CURRENT_MEMORY_USAGE.lock().unwrap();
    while *memory + size > *TOTAL_MEMORY {
        memory = MEMORY_CONDVAR.wait(memory).unwrap();
    }
    info!("memory acquired");
    *memory += size;
    MemoryGuard(size)
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        let mut memory = CURRENT_MEMORY_USAGE.lock().unwrap();
        *memory -= self.0;
        drop(memory);
        MEMORY_CONDVAR.notify_all();
    }
}

#[allow(dead_code)]
pub fn init_logger_stderr(filter: LevelFilter) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(filter.into())
                .from_env_lossy(),
        )
        .with_ansi(true)
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("failed to init logger: {}", e);
        });
}
pub const TIME_TO_LOG: u64 = 15;

/// the main function of the simulator
pub fn main_inner<A, T>(args: A) -> Result<()>
where
    A: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    let file_appender = tracing_appender::rolling::hourly("output/", "spmm.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    init_logger(LevelFilter::INFO, non_blocking);
    ctrlc::set_handler(move || unsafe {
        writeln!(
            io::stderr(),
            "\n------\nCTRL-C received, exiting gracefully"
        )
        .unwrap();
        writeln!(
            io::stderr(),
            "the simulator will stop after the current iteration or wthin {TIME_TO_LOG} secs",
        )
        .unwrap();
        CTRL_C = true;
    })
    .unwrap();

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
            cli::AnalyzeType::NnzDraw => {
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

                analysis::analyze_nnz_gearbox::analyze_nnz_spmm(&config);
                info!("time elapsed: {:?}", current_time.elapsed());
            }
            cli::AnalyzeType::GearboxOrigin => {
                let current_time = std::time::Instant::now();
                info!("analyze with config: {:?}", config);
                let config = Config::new(config);

                let stem = config.output_path.file_stem().unwrap();
                let externsion = config.output_path.extension().unwrap();
                let new_file_name = format!(
                    "{}_gearbox_origin.{}",
                    stem.to_str().unwrap(),
                    externsion.to_str().unwrap()
                );
                let dir_name = config.output_path.parent().unwrap();
                let new_path = dir_name.join(new_file_name);
                info!("the result will be written to {:?}", new_path);

                let gearbox_result = analysis::analyze_gearbox_origin::analyze_gearbox(&config);
                serde_json::to_writer(BufWriter::new(File::create(new_path)?), &gearbox_result)?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
            cli::AnalyzeType::GearboxOriginAll => {
                let current_time = std::time::Instant::now();
                info!("analyze with config: {:?}", config);
                let config = Config::new(config);

                let stem = config.output_path.file_stem().unwrap();
                let externsion = config.output_path.extension().unwrap();
                let new_file_name = format!(
                    "{}_gearbox_origin_all.{}",
                    stem.to_str().unwrap(),
                    externsion.to_str().unwrap()
                );
                let dir_name = config.output_path.parent().unwrap();
                let new_path = dir_name.join(new_file_name);
                info!("the result will be written to {:?}", new_path);

                let gearbox_result = analysis::analyze_gearbox_origin_all::analyze_gearbox(&config);
                serde_json::to_writer(BufWriter::new(File::create(new_path)?), &gearbox_result)?;
                info!("time elapsed: {:?}", current_time.elapsed());
            }
        },
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, Mutex};

    use sprs::{num_kinds::Pattern, CsMat};
    use tracing::{info, metadata::LevelFilter};

    use crate::{acquire_memory, init_logger_stderr, main_inner, pim::config::Config, Simulator};

    #[test]
    fn it_works() {
        for i in (1..=5).rev() {
            println!("{}", i);
        }
        let a = '\x00' as u32;
        println!("{}", a);
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

    #[test]
    fn test_condvar() -> eyre::Result<()> {
        let var = Arc::new(Mutex::new(0));
        let cond = Arc::new(Condvar::new());
        let threads = (0..10)
            .into_iter()
            .map(|i| {
                let var = var.clone();
                let cond = cond.clone();
                std::thread::spawn(move || {
                    let mut var = var.lock().unwrap();
                    *var += 1;
                    println!("thread {} is waiting", i);
                    var = cond.wait(var).unwrap();
                    println!("thread {} is notified", i);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    drop(var);
                })
            })
            .collect::<Vec<_>>();
        std::thread::sleep(std::time::Duration::from_secs(5));
        cond.notify_all();
        threads.into_iter().for_each(|t| t.join().unwrap());
        Ok(())
    }

    #[test]
    fn test_memory_acquire() -> eyre::Result<()> {
        init_logger_stderr(LevelFilter::INFO);
        std::env::set_var("MEMORY_LIMIT", "100");
        let threads = [40, 50, 60, 70, 80]
            .into_iter()
            .enumerate()
            .map(|(i, size)| {
                std::thread::spawn(move || {
                    let _span = tracing::info_span!("thread", i).entered();
                    info!("thread {} start running", i);
                    let _memory = acquire_memory(size);
                    info!("thread {} is runing!", i);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    info!("thread {} finished", i);
                })
            })
            .collect::<Vec<_>>();
        threads.into_iter().for_each(|t| t.join().unwrap());
        Ok(())
    }
}
