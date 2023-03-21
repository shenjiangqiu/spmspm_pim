//! a library for creating pim simulator
#![deny(unsafe_code)]
// #![warn(missing_docs)]
pub mod analysis;
pub mod pim;
use clap::Parser;
use cli::{AnalyzeArgs, Cli, RunArgs};
use eyre::Result;
use once_cell::sync::Lazy;
pub use pim::Simulator;
pub mod tools;
use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, Read},
    net::TcpListener,
    sync::{Condvar, Mutex},
};
use sysinfo::SystemExt;
use tracing::{error, info, metadata::LevelFilter};
use tracing_subscriber::fmt::MakeWriter;

use crate::tools::stop_signal;
pub mod cli;
pub mod draw;

#[allow(dead_code)]
pub fn init_logger_info() {
    init_logger_with_ansi(LevelFilter::INFO, io::stderr, true);
}

#[allow(dead_code)]
pub fn init_logger_debug() {
    init_logger_with_ansi(LevelFilter::DEBUG, io::stderr, true);
}

#[allow(dead_code)]
pub fn init_logger_with_ansi(
    filter: LevelFilter,
    writter: impl for<'writer> MakeWriter<'writer> + 'static + Send + Sync,
    ansi: bool,
) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(filter.into())
                .from_env_lossy(),
        )
        .with_writer(writter)
        .with_ansi(ansi)
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("failed to init logger: {}", e);
        });
}

#[allow(dead_code)]
pub fn init_logger(
    filter: LevelFilter,
    writter: impl for<'writer> MakeWriter<'writer> + 'static + Send + Sync,
) {
    init_logger_with_ansi(filter, writter, false);
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

static TOTAL_MEMORY: Lazy<usize> = Lazy::new(parse_memory_limit);
pub struct MemoryGuard(usize);

///acquire memory, if the memory limit is exceeded, wait until the memory is released
///
///
///  the returned value is a guard, when the guard is dropped, the memory is released
#[must_use]
pub fn acquire_memory(size: usize) -> MemoryGuard {
    info!("trying to acquire memory: {} bytes", size);
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

#[must_use]
pub fn acquire_memory_sections(size: &[usize]) -> Vec<MemoryGuard> {
    assert!(!size.is_empty());
    let total_size = size.iter().sum::<usize>();
    info!("trying to acquire memory: {} bytes", total_size);
    if total_size > *TOTAL_MEMORY {
        panic!("memory limit exceeded");
    }
    let mut memory = CURRENT_MEMORY_USAGE.lock().unwrap();
    while *memory + total_size > *TOTAL_MEMORY {
        memory = MEMORY_CONDVAR.wait(memory).unwrap();
    }
    info!("memory acquired");
    *memory += total_size;
    size.iter().map(|x| MemoryGuard(*x)).collect()
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

/// listen to the port and wait for the command to stop
/// - the port is written to the file "port"
/// - it will pannic when no port is available or unable to write to the file "port"
fn setup_exit_receiver() {
    std::thread::spawn(|| {
        let listener = TcpListener::bind(":::0").unwrap();
        let port = listener.local_addr().unwrap().port();
        info!("listening on port {}", port);
        println!("listening on port {}", port);
        fs::write("port", port.to_string()).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buffer = 0u32.to_le_bytes();
                    match stream.read_exact(&mut buffer) {
                        Err(e) => {
                            error!("failed to read command: {}", e);
                        }
                        Ok(_) => {
                            let cmd = u32::from_le_bytes(buffer);
                            if cmd == 33 {
                                stop_signal::stop();
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("failed to accept connection: {}", e);
                }
            }
        }
    });
}

/// the main function of the simulator
pub fn main_inner<A, T>(args: A) -> Result<()>
where
    A: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let file_appender = tracing_appender::rolling::hourly("output/", "spmm.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    setup_exit_receiver();
    let cli = Cli::parse_from(args);

    analysis::do_analyze(cli, non_blocking)?;
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
    #[allow(clippy::needless_collect)]
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
    #[allow(clippy::needless_collect)]
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

    #[test]
    fn test_drain() {
        let mut data = vec![1, 2, 3, 4, 5, 6];
        let mut drain_iter = data.drain(1..3);
        for i in &mut drain_iter {
            println!("{}", i);
        }
        drop(drain_iter);
        println!("{:?}", data);
    }
}
