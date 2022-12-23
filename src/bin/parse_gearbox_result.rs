use std::{fmt::Display, fs::File, path::PathBuf};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use spmspm_pim::analysis::analyze_gearbox::GearboxReslt;
use tracing::{info, metadata::LevelFilter};

#[derive(Deserialize)]
struct InputList {
    files: Vec<PathBuf>,
}
fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .try_init()
        .unwrap_or_else(|e| {
            eprintln!("failed to init logger: {}", e);
        });

    let input_list: InputList =
        toml::from_str(&std::fs::read_to_string("gearbox_result_list.toml")?)?;

    for file in input_list.files {
        info!("parsing file: {:?}", file);
        let split_spmm_result: GearboxReslt = serde_json::from_reader(File::open(&file)?)?;
        let mut reports = vec![];
        for graph in split_spmm_result.results {
            info!("graph: {}", graph.name);
            println!("graph: {}", graph.name);
            // for cycle
            let cycle_min_max = graph.subarray_result.iter().map(|s| s.cycle).minmax();
            let cycle_mean = graph.subarray_result.iter().map(|s| s.cycle).sum::<usize>() as f64
                / graph.subarray_result.len() as f64;

            let (cycle_min, cycle_max) = cycle_min_max
                .into_option()
                .ok_or(eyre::eyre!("no result"))?;
            println!(
                "min: {}, max: {}, mean: {}",
                cycle_min, cycle_max, cycle_mean
            );

            // for row_open_cycle
            let row_open_min_max = graph
                .subarray_result
                .iter()
                .map(|s| s.row_open_cycle)
                .minmax();
            let row_open_mean = graph
                .subarray_result
                .iter()
                .map(|s| s.row_open_cycle)
                .sum::<usize>() as f64
                / graph.subarray_result.len() as f64;
            let (row_open_min, row_open_max) = row_open_min_max
                .into_option()
                .ok_or(eyre::eyre!("no result"))?;
            println!(
                "min: {}, max: {}, mean: {}",
                row_open_min, row_open_max, row_open_mean
            );

            // for row_read_cycle
            let row_read_min_max = graph
                .subarray_result
                .iter()
                .map(|s| s.row_read_cycle)
                .minmax();
            let row_read_mean = graph
                .subarray_result
                .iter()
                .map(|s| s.row_read_cycle)
                .sum::<usize>() as f64
                / graph.subarray_result.len() as f64;
            let (row_read_min, row_read_max) = row_read_min_max
                .into_option()
                .ok_or(eyre::eyre!("no result"))?;
            println!(
                "min: {}, max: {}, mean: {}",
                row_read_min, row_read_max, row_read_mean
            );

            // for row_write_cycle
            let row_write_min_max = graph
                .subarray_result
                .iter()
                .map(|s| s.row_write_cycle)
                .minmax();
            let row_write_mean = graph
                .subarray_result
                .iter()
                .map(|s| s.row_write_cycle)
                .sum::<usize>() as f64
                / graph.subarray_result.len() as f64;
            let (row_write_min, row_write_max) = row_write_min_max
                .into_option()
                .ok_or(eyre::eyre!("no result"))?;
            println!(
                "min: {}, max: {}, mean: {}",
                row_write_min, row_write_max, row_write_mean
            );

            // for comp_cycle
            let comp_min_max = graph.subarray_result.iter().map(|s| s.comp_cycle).minmax();
            let comp_mean = graph
                .subarray_result
                .iter()
                .map(|s| s.comp_cycle)
                .sum::<usize>() as f64
                / graph.subarray_result.len() as f64;
            let (comp_min, comp_max) =
                comp_min_max.into_option().ok_or(eyre::eyre!("no result"))?;
            println!("min: {}, max: {}, mean: {}", comp_min, comp_max, comp_mean);

            // now for the ring
            let ring_cycle_min_max = graph.ring_result.iter().map(|s| s.cycle).minmax();
            let ring_cycle_mean = graph.ring_result.iter().map(|s| s.cycle).sum::<usize>() as f64
                / graph.ring_result.len() as f64;
            let (ring_cycle_min, ring_cycle_max) = ring_cycle_min_max
                .into_option()
                .ok_or(eyre::eyre!("no result"))?;
            println!(
                "min: {}, max: {}, mean: {}",
                ring_cycle_min, ring_cycle_max, ring_cycle_mean
            );

            // now for the tsv
            let tsv_cycle_min_max = graph.tsv_result.iter().map(|s| s.cycle).minmax();
            let tsv_cycle_mean = graph.tsv_result.iter().map(|s| s.cycle).sum::<usize>() as f64
                / graph.tsv_result.len() as f64;

            let (tsv_cycle_min, tsv_cycle_max) = tsv_cycle_min_max
                .into_option()
                .ok_or(eyre::eyre!("no result"))?;
            println!(
                "min: {}, max: {}, mean: {}",
                tsv_cycle_min, tsv_cycle_max, tsv_cycle_mean
            );

            // now pakck the result to Report
            let report = Report {
                graph_name: graph.name.clone(),
                cycle: (cycle_min, cycle_max, cycle_mean),
                row_open_cycle: (row_open_min, row_open_max, row_open_mean),
                row_read_cycle: (row_read_min, row_read_max, row_read_mean),
                row_write_cycle: (row_write_min, row_write_max, row_write_mean),
                comp_cycle: (comp_min, comp_max, comp_mean),
                ring_cycle: (ring_cycle_min, ring_cycle_max, ring_cycle_mean),
                tsv_cycle: (tsv_cycle_min, tsv_cycle_max, tsv_cycle_mean),
            };
            reports.push(report);
        }
        // the report file will change the file's suffix to .json
        let report_file = file
            .with_extension("json")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let full_path = "reports/".to_owned() + &report_file;
        serde_json::to_writer_pretty(File::create(full_path)?, &reports)?;
    }

    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
struct Report {
    graph_name: String,
    cycle: (usize, usize, f64),
    row_open_cycle: (usize, usize, f64),
    row_read_cycle: (usize, usize, f64),
    row_write_cycle: (usize, usize, f64),
    comp_cycle: (usize, usize, f64),
    ring_cycle: (usize, usize, f64),
    tsv_cycle: (usize, usize, f64),
}
