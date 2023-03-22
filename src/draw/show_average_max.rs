use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    io::{BufReader, Write},
    ops::Deref,
    path::PathBuf,
};

use eyre::Context;
use plotters::{coord::Shift, prelude::*};
use rayon::prelude::*;
use statrs::statistics::Statistics;
use tracing::info;

use super::DrawFn;
use crate::{
    analysis::three_stages::analyze_refined_new_mapping, cli::ExecResult, draw::draw_data_with_size,
};

pub fn draw(result: ExecResult) -> eyre::Result<()> {
    let ExecResult {
        result_file,
        output,
    } = result;
    let output_path = output.unwrap_or_else(|| "console.png".into());
    let gearbox_result = result_file.unwrap_or_else(|| "output/gearbox_all_multi".into());
    if !gearbox_result.is_dir() {
        return Err(eyre::eyre!("gearbox result is not a directory"));
    }
    let files = std::fs::read_dir(gearbox_result)?;
    let files = files.filter_map(|dir_entry| {
        // file name looks like: output/gearbox_out_v2_all_gearbox_origin_all_2_0.00005.json
        let file_name_regex =
            regex::Regex::new(r"gearbox_out_v2_all_gearbox_origin_all_(\d+)_(\d+\.\d+)\.json")
                .unwrap();

        // create a closure, if the file name matches the reges, return Some((batch, topk, file_name)), else return an error
        let is_start_with_gearbox = |file_name: PathBuf| -> eyre::Result<(usize, f32, PathBuf)> {
            let captures = file_name_regex.captures(
                file_name
                    .file_name()
                    .ok_or(eyre::eyre!("failed to get file name"))?
                    .to_str()
                    .ok_or(eyre::eyre!("failed to get file name"))?,
            );
            if let Some(captures) = captures {
                let batch = captures.get(1).unwrap().as_str().parse::<usize>()?;
                let topk = captures.get(2).unwrap().as_str().parse::<f32>()?;
                Ok((batch, topk, file_name))
            } else {
                Err(eyre::eyre!("failed to parse file name"))
            }
        };

        let file_name = dir_entry.unwrap().path();
        is_start_with_gearbox(file_name).ok()
    });
    let results: Vec<_> = files
        .par_bridge()
        .map(|(batch, tokp, file_name)| -> eyre::Result<_> {
            info!("parse the result file: {:?}", file_name);
            let result: Vec<analyze_refined_new_mapping::SingleResult> =
                serde_json::from_reader(BufReader::new(File::open(&file_name)?)).wrap_err(
                    eyre::eyre!("cannot parse the file to gearbox result for file: {file_name:?}"),
                )?;
            Ok((batch, tokp, result))
        })
        .map(|r| r.unwrap().2)
        .collect();

    info!(
        "finished parse the result file: there are total {} configs, each has {} graphs",
        results.len(),
        results[0].len()
    );

    let transposed_result = analyze_refined_new_mapping::transpose2(results);
    info!(
        "finished transpose the result file: there are total {} graphs, each has {} configs",
        transposed_result.len(),
        transposed_result[0].len()
    );
    info!("start draw the result");
    draw_data_with_size::<_, GearboxAllDrawer>(&output_path, &transposed_result, (1920, 1080))?;

    Ok(())
}

#[derive(PartialEq)]
struct TopK(f32);
impl Eq for TopK {}

impl PartialOrd for TopK {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.partial_cmp(&other.0).unwrap())
    }
}
impl Ord for TopK {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}
impl Deref for TopK {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct GearboxAllDrawer;
impl DrawFn for GearboxAllDrawer {
    type DATA = Vec<Vec<analyze_refined_new_mapping::SingleResult>>;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        _root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        println!("local_max local_average local_disp_max local_disp_average dispatching_max dispatching_average remote_max remote_average");
        let mut local_total_result = Vec::new();
        let mut local_dispatcher_total_result = Vec::new();
        let mut dispathing_total_result = Vec::new();
        let mut remote_total_result = Vec::new();
        for graph in data.iter() {
            let batches = graph.iter().map(|x| x.batch).collect::<BTreeSet<_>>();

            let topks = graph.iter().map(|x| TopK(x.topk)).collect::<BTreeSet<_>>();
            let maped_results = graph
                .iter()
                .enumerate()
                .map(|(i, x)| ((x.batch, (TopK(x.topk))), i))
                .collect::<BTreeMap<_, _>>();

            info!("draw graph {}", graph[0].name);
            let mut local_result = Vec::new();
            let mut local_dispatcher_result = Vec::new();
            let mut dispathing_result = Vec::new();
            let mut remote_result = Vec::new();
            for (_i, &TopK(topk)) in topks.iter().enumerate() {
                // the max cycle and the average cycle
                for (_j, &batch) in batches.iter().enumerate() {
                    let result = &graph[*maped_results.get(&(batch, TopK(topk))).unwrap()];
                    let top_1000_distribution = &result.total_result.top_1000_distribution;
                    let local_average =
                        top_1000_distribution.iter().map(|x| x.local_average).mean();
                    let local_max = top_1000_distribution
                        .iter()
                        .map(|x| x.local_max as f64)
                        .mean();

                    let local_disp_average = top_1000_distribution
                        .iter()
                        .map(|x| x.local_dispatcher_average)
                        .mean();
                    let local_disp_max = top_1000_distribution
                        .iter()
                        .map(|x| x.local_dispatcher_max as f64)
                        .mean();

                    let dispatching_average = top_1000_distribution
                        .iter()
                        .map(|x| x.dispatching_average)
                        .mean();
                    let dispatching_max = top_1000_distribution
                        .iter()
                        .map(|x| x.dispatching_max as f64)
                        .mean();

                    let remote_average = top_1000_distribution
                        .iter()
                        .map(|x| x.remote_average)
                        .mean();
                    let remote_max = top_1000_distribution
                        .iter()
                        .map(|x| x.remote_max as f64)
                        .mean();

                    let local_rate = local_max / local_average;
                    let local_disp_rate = local_disp_max / local_disp_average;
                    let dispatching_rate = dispatching_max / dispatching_average;
                    let remote_rate = remote_max / remote_average;
                    local_result.push(local_rate);
                    local_dispatcher_result.push(local_disp_rate);
                    dispathing_result.push(dispatching_rate);
                    remote_result.push(remote_rate);
                }
            }
            local_total_result.push(local_result);
            local_dispatcher_total_result.push(local_dispatcher_result);
            dispathing_total_result.push(dispathing_result);
            remote_total_result.push(remote_result);
        }
        // wirte the result to file
        let mut local_total_result_file = File::create("local_total_result.txt")?;
        let mut local_dispatcher_total_result_file =
            File::create("local_dispatcher_total_result.txt")?;
        let mut dispathing_total_result_file = File::create("dispathing_total_result.txt")?;
        let mut remote_total_result_file = File::create("remote_total_result.txt")?;
        let from_vec_to_string = |data: Vec<Vec<f64>>| {
            data.into_iter()
                .map(|graph_result| {
                    graph_result
                        .into_iter()
                        .fold("".to_string(), |acc, data| format!("{acc} {}", data))
                })
                .fold("".to_string(), |acc, data| format!("{acc}\n{data}"))
        };
        let out = from_vec_to_string(local_total_result);
        local_total_result_file.write_all(out.as_bytes())?;
        let out = from_vec_to_string(local_dispatcher_total_result);
        local_dispatcher_total_result_file.write_all(out.as_bytes())?;
        let out = from_vec_to_string(dispathing_total_result);
        dispathing_total_result_file.write_all(out.as_bytes())?;
        let out = from_vec_to_string(remote_total_result);
        remote_total_result_file.write_all(out.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_div() {
        let result = 1. / 0.;
        println!("{}", result)
    }
}
