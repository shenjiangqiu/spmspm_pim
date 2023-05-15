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
use crate::{analysis::analyze_gearbox_origin_all_v2, cli::ExecResult, draw::draw_data_with_size};

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
            let result: Vec<analyze_gearbox_origin_all_v2::SingleResult> =
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

    let transposed_result = analyze_gearbox_origin_all_v2::transpose2(results);
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
    type DATA = Vec<Vec<analyze_gearbox_origin_all_v2::SingleResult>>;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let charts = root.split_evenly((5, 5));
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
            let max_cycle = graph
                .iter()
                .map(|x| {
                    x.total_result
                        .global_max_acc_cycle
                        .max(x.total_result.global_max_ring_buffer)
                        .max(x.total_result.global_max_real_local)
                        + x.total_result
                            .gloabl_max_acc_ring
                            .max(x.total_result.global_max_acc_tsv)
                        + x.total_result.global_max_acc_cycle_remote
                })
                .max()
                .unwrap();
            let y_spec = 0..max_cycle;
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
                    let total_cycle = result.total_result.global_max_acc_cycle;
                    let total_ring = result.total_result.gloabl_max_acc_ring;
                    let total_tsv = result.total_result.global_max_acc_tsv;
                    let total_remote = result.total_result.global_max_acc_cycle_remote;
                    let total_disp_real = result.total_result.global_max_real_local;
                    let total_disp = result.total_result.global_max_ring_buffer;
                    draw_lines(
                        &mut chart,
                        i as f32,
                        j as f32,
                        total_cycle,
                        total_ring,
                        total_tsv,
                        total_remote,
                    )?;
                    draw_lines(
                        &mut chart,
                        i as f32 + 0.2,
                        j as f32 + 0.2,
                        total_disp_real,
                        total_ring,
                        total_tsv,
                        total_remote,
                    )?;
                    draw_lines(
                        &mut chart,
                        i as f32 + 0.4,
                        j as f32 + 0.4,
                        total_disp,
                        total_ring,
                        total_tsv,
                        total_remote,
                    )?;
                }
            }
        }

        root.present()?;
        Ok(())
    }
}

fn draw_lines<'a, DB: plotters::prelude::DrawingBackend + 'a>(
    chart: &mut ChartContext<
        DB,
        plotters::coord::ranged3d::Cartesian3d<
            plotters::coord::types::RangedCoordf32,
            plotters::coord::types::RangedCoordusize,
            plotters::coord::types::RangedCoordf32,
        >,
    >,
    i: f32,
    j: f32,
    total_cycle: usize,
    total_ring: usize,
    total_tsv: usize,
    total_remote: usize,
) -> Result<(), Box<dyn Error + 'a>> {
    chart.draw_series(LineSeries::new(
        [
            (i, 0, j),
            (i, total_cycle, j),
            (i + 0.2, total_cycle, j + 0.2),
        ],
        BLUE,
    ))?;
    chart.draw_series(LineSeries::new(
        [
            (i, total_cycle, j),
            (i, total_cycle + total_ring.max(total_tsv), j),
            (i + 0.2, total_cycle + total_ring.max(total_tsv), j + 0.2),
        ],
        GREEN,
    ))?;
    chart.draw_series(LineSeries::new(
        [
            (i, total_cycle + total_ring.max(total_tsv), j),
            (i, total_cycle + total_ring.max(total_tsv) + total_remote, j),
            (
                i + 0.2,
                total_cycle + total_ring.max(total_tsv) + total_remote,
                j + 0.2,
            ),
        ],
        RED,
    ))?;
    Ok(())
}
