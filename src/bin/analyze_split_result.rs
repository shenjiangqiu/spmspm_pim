use std::{fmt::Display, fs::File};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use spmspm_pim::analysis::analyze_split_spmm::SplitAnalyzeResult;

#[derive(Debug, Serialize, Deserialize)]
struct Report {
    name: String,
    nnz_stats: spmspm_pim::analysis::split::NnzStats,
    min_cycle: u64,
    max_cycle: u64,
    mean_cycle: f64,

    mean_comp: f64,
    mean_open: f64,
    mean_open_no_overlap: f64,
    mean_temp_read: f64,
    mean_temp_write: f64,
    mean_input_read: f64,
    row_open_bytes: f64,
    used_bytes: f64,

    input_read_bytes: f64,
    input_read_times: f64,
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "graph: {}", self.name)?;
        writeln!(f, "nnz: {:?}", self.nnz_stats)?;
        writeln!(
            f,
            "min: {:.2}, max: {:.2}, mean: {:.2}",
            self.min_cycle, self.max_cycle, self.mean_cycle
        )?;
        writeln!(f,
            "mean_comp: {:.2}-{:.2}%, mean_open: {:.2}-{:.2}%, mean_open_no_overlap: {:.2}, mean_temp_read: {:.2}-{:.2}%, mean_temp_write: {:.2}-{:.2}%, mean_input_read: {:.2}-{:.2}%",
            self.mean_comp,self.mean_comp/self.mean_cycle*100.,
            self.mean_open,self.mean_open/self.mean_cycle*100.,
            self.mean_open_no_overlap,
            self.mean_temp_read,self.mean_temp_read/self.mean_open_no_overlap*100., 
            self.mean_temp_write,self.mean_temp_write/self.mean_open_no_overlap*100.,
            self.mean_input_read,self.mean_input_read/self.mean_open_no_overlap*100.
        )?;
        writeln!(
            f,
            "row_open_bytes: {:.2}, used_bytes: {:.2}, use_rate: {:.2}%",
            self.row_open_bytes,
            self.used_bytes,
            self.used_bytes / self.row_open_bytes * 100.
        )?;
        writeln!(
            f,
            "input_read_bytes: {:.2}, input_read_times: {:.2}, read_per_time: {:.2}",
            self.input_read_bytes,
            self.input_read_times,
            self.input_read_bytes / self.input_read_times
        )?;

        Ok(())
    }
}

fn main() -> eyre::Result<()> {
    let split_spmm_result: SplitAnalyzeResult =
        serde_json::from_reader(File::open("split_spmm.json")?)?;
    for graph in split_spmm_result.results {
        println!("graph: {}", graph.name);
        println!("nnz: {:?}", graph.nnz_stats);
        let min_max = graph.graph_result.iter().map(|s| s.cycle).minmax();
        let mean = graph.graph_result.iter().map(|s| s.cycle).sum::<u64>() as f64
            / graph.graph_result.len() as f64;
        let (min, max) = min_max.into_option().ok_or(eyre::eyre!("no result"))?;
        println!("min: {}, max: {}, mean: {}", min, max, mean);
        let num_patitions = graph.graph_result.len() as f64;
        let mean_comp = graph
            .graph_result
            .iter()
            .map(|s| s.compute_cycle)
            .sum::<u64>() as f64
            / num_patitions;
        let mean_open =
            graph.graph_result.iter().map(|s| s.row_open).sum::<u64>() as f64 / num_patitions;
        let mean_open_no_overlap = graph
            .graph_result
            .iter()
            .map(|s| s.row_open_no_overlap)
            .sum::<u64>() as f64
            / num_patitions;
        let mean_temp_read = graph
            .graph_result
            .iter()
            .map(|s| s.temp_result_read)
            .sum::<u64>() as f64
            / num_patitions;
        let mean_temp_write = graph
            .graph_result
            .iter()
            .map(|s| s.final_result_write)
            .sum::<u64>() as f64
            / num_patitions;
        let mean_input_read = graph
            .graph_result
            .iter()
            .map(|s| s.matrix_b_read)
            .sum::<u64>() as f64
            / num_patitions;
        let row_open_bytes = graph
            .graph_result
            .iter()
            .map(|s| s.row_open_bytes)
            .sum::<usize>() as f64
            / num_patitions;
        let row_read_bytes = graph
            .graph_result
            .iter()
            .map(|s| s.used_bytes)
            .sum::<usize>() as f64
            / num_patitions;
        let input_read_bytes = graph
            .graph_result
            .iter()
            .map(|s| s.input_read_bytes)
            .sum::<usize>() as f64
            / num_patitions;
        let input_read_times = graph
            .graph_result
            .iter()
            .map(|s| s.input_read_times)
            .sum::<usize>() as f64
            / num_patitions;
        let report = Report {
            name: graph.name,
            nnz_stats: graph.nnz_stats,
            min_cycle: min,
            max_cycle: max,
            mean_cycle: mean,
            mean_comp,
            mean_open,
            mean_open_no_overlap,
            mean_temp_read,
            mean_temp_write,
            mean_input_read,
            row_open_bytes,
            used_bytes: row_read_bytes,
            input_read_bytes,
            input_read_times,
        };
        println!("{}", report);
    }
    Ok(())
}
