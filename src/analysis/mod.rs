//! # the analysis module
//! show the key timing and bandwidth
//!

use std::fmt::Debug;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::pim::configv2::ConfigV2;
use crate::{cli, pim::config::Config};
use crate::{draw, init_logger, RunArgs, Simulator};
use crate::{init_logger_stderr, AnalyzeArgs};
use serde::Serialize;
use tracing::info;
use tracing::metadata::LevelFilter;

use self::three_stages::{analyze_refined_dispatcher_overflow, analyze_refined_distribution};
pub mod analyze_channel;
pub mod analyze_gearbox;
pub mod analyze_gearbox_origin;
pub mod analyze_gearbox_origin_all;
pub mod analyze_gearbox_origin_all_v2;
pub mod analyze_gearbox_origin_all_v2_overflow;
pub mod analyze_gearbox_parallel;
pub(crate) mod analyze_nnz;
pub mod analyze_nnz_gearbox;
pub(crate) mod analyze_nnz_native;

pub mod analyze_split_spmm;
pub mod compute_merger_cycle;
pub mod event;
pub mod mergered_stream;
pub mod overlap;
pub mod partition;
pub mod schedule_window;
pub mod sequential_event_sim;
pub mod split;
pub mod three_stages;

pub mod analyze_gearbox_overflow_and_traffic;
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

pub fn do_analyze(
    cli: crate::Cli,
    non_blocking: tracing_appender::non_blocking::NonBlocking,
) -> Result<(), eyre::ErrReport> {
    match cli.subcmd {
        cli::Operation::Run(RunArgs { config }) => {
            init_logger(LevelFilter::INFO, non_blocking);
            println!("run with config: {:?}", config);
            let config = Config::new(config);
            info!("building simulator");
            let mut simulator = Simulator::new(&config);
            info!("start running simulator");

            simulator.run(&config);
        }

        cli::Operation::Analyze(AnalyzeArgs { analyze, config }) => {
            init_logger(LevelFilter::INFO, non_blocking);
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
                    let config_v2 = ConfigV2::new(config);
                    do_analyze_by_batch_and_topk(
                        &config_v2,
                        &config_v2.output_path,
                        analyze_refined_distribution::analyze_gearbox,
                    )?;
                }
            }
        }
        cli::Operation::Draw(draw_args) => {
            init_logger_stderr(LevelFilter::INFO);
            draw::draw_with_type(draw_args.subcmd)?
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pim::config::Config;
    #[test]
    fn test_print_all_stat() {
        let config: Config =
            toml::from_str(std::fs::read_to_string("ddr4.toml").unwrap().as_str()).unwrap();
        print_all_stats(&config);
    }
}
