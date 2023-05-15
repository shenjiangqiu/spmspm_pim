//! # the analysis module
//! show the key timing and bandwidth
pub mod evil_filter;
pub mod remap_analyze;
pub mod translate_mapping;

pub mod results;
pub mod stats;
pub mod traits;
use crate::cli::LogType;
use crate::pim::configv2::{ConfigV2, ConfigV3};
use crate::{cli, pim::config::Config};
use crate::{draw, init_logger, RunArgs, Simulator};
use crate::{init_logger_stderr, AnalyzeArgs};
use serde::Serialize;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::RwLock;
use tracing::info;
use tracing::metadata::LevelFilter;
pub mod mapping;
use self::remap_analyze::row_cycle::*;
use self::three_stages::analyze_refined_dispatcher_overflow;
pub mod old;
pub use old::*;
pub mod three_stages;
/// (From Port, Next Port. TargetRing, TargetPort)
pub type RingTask = (RingPort, RingPort, (RingId, RingPort));
pub type RingTasksInASubarray = Vec<RingTask>;
pub type RingTasksInABank = Vec<RingTasksInASubarray>;
pub type RingTasksInAllBanks = Vec<RingTasksInABank>;

pub const EVIL_RATE: f32 = 0.0005;

pub(self) static TOTAL_FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);
pub(self) static TOTAL_TASKS: RwLock<usize> = RwLock::new(0);
pub fn print_all_stats(config: &Config) {
    let single_task_overlap_stat = overlap::compute_single_task_overlap_stat(config);

    for stat in single_task_overlap_stat {
        println!("graph: {}", stat.graph);
        stat.print();
    }

    let lock_task_overlap_stat = sequential_event_sim::compute_lock_task_overlap_stat(config);

    for stat in lock_task_overlap_stat {
        println!("graph: {}", stat.graph);
        stat.print();
    }
}
pub fn transpose2<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

pub fn do_analyze(
    cli: crate::Cli,
    non_blocking: tracing_appender::non_blocking::NonBlocking,
) -> Result<(), eyre::ErrReport> {
    match cli.log_type.unwrap_or(LogType::File) {
        LogType::File => {
            init_logger(LevelFilter::INFO, non_blocking);
        }
        LogType::Stderr => {
            init_logger_stderr(LevelFilter::INFO);
        }
    }
    match cli.subcmd {
        cli::Operation::Run(RunArgs { config }) => {
            println!("run with config: {:?}", config);
            let config = Config::new(config);
            info!("building simulator");
            let mut simulator = Simulator::new(&config);
            info!("start running simulator");

            simulator.run(&config);
        }

        cli::Operation::Analyze(AnalyzeArgs { analyze, config }) => {
            match analyze {
                cli::AnalyzeType::All => {
                    println!("analyze with config: {:?}", config);
                    let config = Config::new(config);
                    print_all_stats(&config);
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

                    let split_result = analyze_split_spmm::analyze_split_spmm(&config);
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

                    let gearbox_result = analyze_gearbox::analyze_gearbox(&config);
                    serde_json::to_writer(
                        BufWriter::new(File::create(new_path)?),
                        &gearbox_result,
                    )?;
                    info!("time elapsed: {:?}", current_time.elapsed());
                }
                cli::AnalyzeType::Nnz => {
                    let current_time = std::time::Instant::now();
                    info!("analyze with config: {:?}", config);
                    let config = Config::new(config);
                    let nnz_result = analyze_nnz::analyze_nnz_spmm(&config);
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
                    let nnz_result = analyze_nnz_native::analyze_nnz_spmm(&config);
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

                    let gearbox_result = analyze_gearbox_parallel::analyze_gearbox(&config);
                    serde_json::to_writer(
                        BufWriter::new(File::create(new_path)?),
                        &gearbox_result,
                    )?;
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

                    analyze_nnz_gearbox::analyze_nnz_spmm(&config);
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

                    let gearbox_result = analyze_gearbox_origin::analyze_gearbox(&config);
                    serde_json::to_writer(
                        BufWriter::new(File::create(new_path)?),
                        &gearbox_result,
                    )?;
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

                    let gearbox_result = analyze_gearbox_origin_all::analyze_gearbox(&config);
                    serde_json::to_writer(
                        BufWriter::new(File::create(new_path)?),
                        &gearbox_result,
                    )?;
                    info!("time elapsed: {:?}", current_time.elapsed());
                }
                cli::AnalyzeType::GearboxOriginAllV2 => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        analyze_gearbox_origin_all_v2::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::GearboxOriginAllV2OverFlow => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        analyze_gearbox_origin_all_v2_overflow::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::GearboxOverflowTraffic => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        analyze_gearbox_overflow_and_traffic::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::AnalyzeChannel => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        analyze_channel::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::AnalyzeRefinedGearbox => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        three_stages::analyze_refined_gearbox::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::AnalyzeRefinedGearboxDispatchOverflow => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        analyze_refined_dispatcher_overflow::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::AnalyzeRefinedDistribution => {
                    todo!()
                }
                cli::AnalyzeType::AnalyzeRefinedNewMapping => {
                    todo!()
                }
                cli::AnalyzeType::AnalyzeBankTrace => {
                    todo!()
                }
                cli::AnalyzeType::AnalyzeBankTraceAll => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        three_stages::analyze_refined_bank_trace_all::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::AnalyzeRefinedNewMappingCycle => {
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        three_stages::analyze_refined_cycle::analyze_gearbox,
                    )?;
                }
                cli::AnalyzeType::AnalyzeRealOneHotJump => {
                    // let config_v2 = ConfigV2::new(config);
                    // do_analyze_by_batch_and_topk(&config_v2, &config_v2.output_path, todo!())?;
                }
                cli::AnalyzeType::NewAnalysis => {
                    let config_v2 = ConfigV3::new(config);
                    remap_analyze::run_simulation(config_v2)?;
                }
            }
        }
        cli::Operation::Draw(draw_args) => draw::draw_with_type(draw_args.subcmd)?,
    };
    Ok(())
}

fn do_analyze_by_batch_and_topk<
    C: Debug,
    F: Fn(&C) -> Vec<((usize, f32), Vec<R>)>,
    R: Serialize,
>(
    config: &C,
    output_path: &Path,
    f: F,
) -> Result<(), eyre::ErrReport> {
    let current_time = std::time::Instant::now();
    info!("analyze with config: {:?}", config);
    let stem = output_path.file_stem().unwrap();
    let externsion = output_path.extension().unwrap();
    let dir_name = output_path.parent().unwrap();
    let gearbox_result = f(config);
    for ((batch, topk), result) in gearbox_result {
        let new_file_name = format!(
            "{}_gearbox_origin_all_{batch}_{topk}.{}",
            stem.to_str().unwrap(),
            externsion.to_str().unwrap()
        );
        let new_path = dir_name.join(new_file_name);
        info!("the result will be written to {:?}", new_path);
        serde_json::to_writer(BufWriter::new(File::create(new_path)?), &result)?;
    }
    info!("time elapsed: {:?}", current_time.elapsed());
    Ok(())
}
