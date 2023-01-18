use std::{error::Error, fs::File, io::BufReader, path::PathBuf};

use plotters::{
    coord::Shift,
    prelude::*,
    style::full_palette::{GREY, PINK},
};
use tracing::info;

use crate::{analysis::analyze_split_spmm::SplitAnalyzeResult, cli::ExecResult, draw::get_ext};

use super::{draw_data, DrawFn};

struct CycleDrawer;

impl DrawFn for CycleDrawer {
    type DATA = SplitAnalyzeResult;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        draw_cycle_dist_rec(root, data)
    }
}

/// draw the cycle distribution of the split result
pub fn draw_cycle_dist(args: ExecResult) -> eyre::Result<()> {
    let ExecResult {
        result_file,
        output,
    } = args;
    let output_path = output.unwrap_or_else(|| "output/cycle_dist.svg".into());
    let split_result =
        result_file.unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_result)?))?;
    // generate the box plot for each graph
    info!("{:?}", get_ext(&output_path));
    draw_data::<_, CycleDrawer>(&output_path, &split_result)?;

    Ok(())
}

fn draw_cycle_dist_rec<'a, DB: DrawingBackend + 'a>(
    root: DrawingArea<DB, Shift>,
    result: &SplitAnalyzeResult,
) -> Result<(), Box<dyn Error + 'a>> {
    let charts = root.split_evenly((4, 5));
    for (graph, chart) in result.results.iter().zip(charts) {
        info!("draw graph {}", graph.name);
        // first get the min_max for the cycles for each bank
        let num_partition = graph.graph_result.len() as u64;

        let cycle = graph.graph_result.iter().map(|x| x.cycle).sum::<u64>() / num_partition;
        let total_cycle_fix_empty_meta = graph
            .graph_result
            .iter()
            .map(|x| x.total_cycle_fix_empty_meta)
            .sum::<u64>()
            / num_partition;
        let total_cycle_ignore_empty_meta = graph
            .graph_result
            .iter()
            .map(|x| x.total_cycle_ignore_empty_meta)
            .sum::<u64>()
            / num_partition;
        let total_cycle_ignore_meta = graph
            .graph_result
            .iter()
            .map(|x| x.total_cycle_ignore_meta)
            .sum::<u64>()
            / num_partition;
        let meta_cycle =
            graph.graph_result.iter().map(|x| x.meta_cycle).sum::<u64>() / num_partition;
        let fix_empty_meta_cycle = graph
            .graph_result
            .iter()
            .map(|x| x.fix_empty_meta_cycle)
            .sum::<u64>()
            / num_partition;
        let ignore_empty_row_meta_cycle = graph
            .graph_result
            .iter()
            .map(|x| x.ignore_empty_row_meta_cycle)
            .sum::<u64>()
            / num_partition;
        let compute_cycle = graph
            .graph_result
            .iter()
            .map(|x| x.compute_cycle)
            .sum::<u64>()
            / num_partition;
        let row_open = graph.graph_result.iter().map(|x| x.row_open).sum::<u64>() / num_partition;
        let max_cycle = *[
            cycle,
            total_cycle_fix_empty_meta,
            total_cycle_ignore_empty_meta,
            total_cycle_ignore_meta,
            meta_cycle,
            fix_empty_meta_cycle,
            ignore_empty_row_meta_cycle,
            compute_cycle,
            row_open,
        ]
        .iter()
        .max()
        .unwrap();

        let colors = [BLACK, RED, BLUE, GREEN, YELLOW, PINK, GREY, CYAN, MAGENTA];
        let name = PathBuf::from(graph.name.clone());
        let segs = [
            "cycle",
            "total_cycle_fix_empty_meta",
            "total_cycle_ignore_empty_meta",
            "total_cycle_ignore_meta",
            "meta_cycle",
            "fix_empty_meta_cycle",
            "ignore_empty_row_meta_cycle",
            "compute_cycle",
            "row_open",
        ];
        let mut chart = ChartBuilder::on(&chart)
            .caption(
                name.file_name().unwrap().to_str().unwrap(),
                ("sans-serif", 20).into_font(),
            )
            .x_label_area_size(10.percent())
            .y_label_area_size(10.percent())
            .margin(5.percent())
            .build_cartesian_2d(0..max_cycle, segs.into_segmented())?;

        chart.configure_mesh().disable_mesh().draw()?;

        chart.draw_series([
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"cycle")),
                    (cycle, SegmentValue::Exact(&"cycle")),
                ],
                colors[0].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"total_cycle_fix_empty_meta")),
                    (
                        total_cycle_fix_empty_meta,
                        SegmentValue::Exact(&"total_cycle_fix_empty_meta"),
                    ),
                ],
                colors[1].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"total_cycle_ignore_empty_meta")),
                    (
                        total_cycle_ignore_empty_meta,
                        SegmentValue::Exact(&"total_cycle_ignore_empty_meta"),
                    ),
                ],
                colors[2].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"total_cycle_ignore_meta")),
                    (
                        total_cycle_ignore_meta,
                        SegmentValue::Exact(&"total_cycle_ignore_meta"),
                    ),
                ],
                colors[3].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"meta_cycle")),
                    (meta_cycle, SegmentValue::Exact(&"meta_cycle")),
                ],
                colors[4].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"fix_empty_meta_cycle")),
                    (
                        fix_empty_meta_cycle,
                        SegmentValue::Exact(&"fix_empty_meta_cycle"),
                    ),
                ],
                colors[5].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"ignore_empty_row_meta_cycle")),
                    (
                        ignore_empty_row_meta_cycle,
                        SegmentValue::Exact(&"ignore_empty_row_meta_cycle"),
                    ),
                ],
                colors[6].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"compute_cycle")),
                    (compute_cycle, SegmentValue::Exact(&"compute_cycle")),
                ],
                colors[7].mix(0.5).filled(),
            ),
            Rectangle::new(
                [
                    (0, SegmentValue::CenterOf(&"row_open")),
                    (row_open, SegmentValue::Exact(&"row_open")),
                ],
                colors[8].mix(0.5).filled(),
            ),
        ])?;

        chart.configure_series_labels().draw()?;
    }
    root.present()?;
    Ok(())
}
