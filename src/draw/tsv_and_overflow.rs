use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    io::BufReader,
    ops::Deref,
    path::PathBuf,
};

use eyre::Context;
use plotters::{coord::Shift, prelude::*};
use rayon::prelude::*;
use tracing::info;

use super::DrawFn;
use crate::{
    analysis::analyze_gearbox_overflow_and_traffic, cli::ExecResult, draw::draw_data_with_size,
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
            let result: Vec<analyze_gearbox_overflow_and_traffic::SingleResult> =
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

    let transposed_result = analyze_gearbox_overflow_and_traffic::transpose2(results);
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
    type DATA = Vec<Vec<analyze_gearbox_overflow_and_traffic::SingleResult>>;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let graphs = data.len();
        let rows = (graphs as f32).sqrt().ceil() as usize;
        let cols = (graphs + rows - 1) / rows;
        let charts = root.split_evenly((rows, cols));
        for (graph, chart) in data.iter().zip(charts.iter()) {
            let batches = graph.iter().map(|x| x.batch).collect::<BTreeSet<_>>();

            let topks = graph.iter().map(|x| TopK(x.topk)).collect::<BTreeSet<_>>();
            let maped_results = graph
                .iter()
                .enumerate()
                .map(|(i, x)| ((x.batch, (TopK(x.topk))), i))
                .collect::<BTreeMap<_, _>>();

            info!("draw graph {}", graph[0].name);
            let x_spec = 0f32..topks.len() as f32;
            let z_spec = 0f32..batches.len() as f32;

            let y_spec = 0f32..1f32;
            let mut chart = ChartBuilder::on(chart)
                .caption(
                    format!("graph: {}", graph[0].name),
                    ("sans-serif", 10).into_font(),
                )
                .build_cartesian_3d(x_spec, y_spec, z_spec)?;
            chart.with_projection(|mut pd| {
                pd.yaw = 0.5;
                pd.scale = 0.9;
                pd.into_matrix()
            });
            chart
                .configure_axes()
                .light_grid_style(BLACK.mix(0.15))
                .max_light_lines(3)
                .draw()?;

            for (i, &TopK(topk)) in topks.iter().enumerate() {
                for (j, &batch) in batches.iter().enumerate() {
                    let result = &graph[*maped_results.get(&(batch, TopK(topk))).unwrap()];
                    let total_use = result.total_result.global_tsv_base_total;
                    let real_use = result.total_result.global_tsv_base_real;
                    let total_valid = result.total_result.global_tsv_base_max_use_validt;
                    // the cycle
                    let tsv_no_conf_cycle = result.total_result.global_tsv_base_cycle_no_conflict;
                    let tsv_cycle = result.total_result.global_tsv_base_cycle_normal;
                    let use_rate: f32 = real_use as f32 / total_use as f32;
                    let no_conflict_rate: f32 = tsv_no_conf_cycle as f32 / tsv_cycle as f32;
                    let use_rate_valid: f32 = real_use as f32 / total_valid as f32;

                    chart.draw_series(LineSeries::new(
                        [(i as f32, 0f32, j as f32), (i as f32, use_rate, j as f32)],
                        &RED,
                    ))?;
                    chart.draw_series(LineSeries::new(
                        [
                            (i as f32 + 0.1, 0f32, j as f32),
                            (i as f32 + 0.1, no_conflict_rate, j as f32),
                        ],
                        &BLACK,
                    ))?;
                    chart.draw_series(LineSeries::new(
                        [
                            (i as f32 + 0.2, 0f32, j as f32),
                            (i as f32 + 0.2, use_rate_valid, j as f32),
                        ],
                        &GREEN,
                    ))?;
                    let overflow_cycle_12_256 = result.total_result.overflow_count_12_256_overhead;
                    let overflow_cycle_12_512 = result.total_result.overflow_count_12_512_overhead;
                    let overflow_cycle_8_256 = result.total_result.overflow_count_8_256_overhead;
                    let overflow_cycle_8_512 = result.total_result.overflow_count_8_512_overhead;
                    let total_cycle = result.total_result.global_max_real_local
                        + result.total_result.global_max_acc_cycle_remote
                        + result
                            .total_result
                            .global_max_acc_tsv
                            .max(result.total_result.global_max_acc_ring);
                    let overflow_cycle_12_256_rate: f32 = overflow_cycle_12_256 as f32
                        / (total_cycle as f32 + overflow_cycle_12_256 as f32);
                    let overflow_cycle_12_512_rate: f32 = overflow_cycle_12_512 as f32
                        / (total_cycle as f32 + overflow_cycle_12_512 as f32);
                    let overflow_cycle_8_256_rate: f32 = overflow_cycle_8_256 as f32
                        / (total_cycle as f32 + overflow_cycle_8_256 as f32);
                    let overflow_cycle_8_512_rate: f32 = overflow_cycle_8_512 as f32
                        / (total_cycle as f32 + overflow_cycle_8_512 as f32);

                    chart.draw_series(LineSeries::new(
                        [
                            (i as f32 + 0.3, 0f32, j as f32),
                            (i as f32 + 0.3, overflow_cycle_12_256_rate, j as f32),
                        ],
                        &BLUE,
                    ))?;
                    chart.draw_series(LineSeries::new(
                        [
                            (i as f32 + 0.4, 0f32, j as f32),
                            (i as f32 + 0.4, overflow_cycle_12_512_rate, j as f32),
                        ],
                        &BLUE,
                    ))?;
                    chart.draw_series(LineSeries::new(
                        [
                            (i as f32 + 0.5, 0f32, j as f32),
                            (i as f32 + 0.5, overflow_cycle_8_256_rate, j as f32),
                        ],
                        &BLUE,
                    ))?;
                    chart.draw_series(LineSeries::new(
                        [
                            (i as f32 + 0.6, 0f32, j as f32),
                            (i as f32 + 0.6, overflow_cycle_8_512_rate, j as f32),
                        ],
                        &BLUE,
                    ))?;
                }
            }
        }

        root.present()?;
        Ok(())
    }
}
