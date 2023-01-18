use std::{error::Error, fs::File, io::BufReader};

use itertools::Itertools;
use plotters::{
    coord::Shift,
    data::fitting_range,
    prelude::*,
    style::full_palette::{BLUEGREY, PINK},
};
use tracing::info;

use crate::{analysis::analyze_split_spmm::SplitAnalyzeResult, cli::ExecResult};

use super::{draw_data, DrawFn};

struct SplitDrawer;

impl DrawFn for SplitDrawer {
    type DATA = SplitAnalyzeResult;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        draw_box(root, data)
    }
}
pub fn draw_split(args: ExecResult) -> eyre::Result<()> {
    let ExecResult {
        result_file,
        output,
    } = args;
    let output_path = output.unwrap_or_else(|| "console.svg".into());
    let split_result =
        result_file.unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_result)?))?;
    // generate the box plot for each graph
    draw_data::<_, SplitDrawer>(&output_path, &split_result)?;
    Ok(())
}
fn draw_box<'a, DB: DrawingBackend + 'a>(
    root: DrawingArea<DB, Shift>,
    result: &SplitAnalyzeResult,
) -> Result<(), Box<dyn Error + 'a>> {
    root.fill(&WHITE)?;
    let charts = root.split_evenly((4, 5));
    for (graph, chart) in result.results.iter().zip(charts) {
        info!("draw graph {}", graph.name);
        // first get the min_max for the cycles for each bank
        let value_range = fitting_range(
            graph
                .graph_result
                .iter()
                .flat_map(|x| {
                    [
                        x.cycle,
                        x.meta_cycle,
                        x.compute_cycle,
                        x.row_open,
                        x.ignore_empty_row_meta_cycle,
                    ]
                })
                .collect_vec()
                .as_slice(),
        );

        let types = [
            "cycle".to_string(),
            "total_cycle_ignore_empty".to_string(),
            "total_cycle_fix_empty".to_string(),
            "ignore_empty_row_meta_cycle".to_string(),
            "fix_empty_row_meta_cycle".to_string(),
            "meta_cycle".to_string(),
            "compute_cycle".to_string(),
            "row_open".to_string(),
        ];
        let colors = [
            RGBColor(255, 0, 0),
            RGBColor(0, 255, 0),
            RGBColor(0, 0, 255),
            RGBColor(255, 255, 0),
            RGBColor(255, 0, 255),
            PINK,
            BLACK,
            BLUEGREY,
        ];
        let segments = types.clone();
        let range_size = value_range.end - value_range.start;
        let mut chart = ChartBuilder::on(&chart)
            .caption(graph.name.clone(), ("sans-serif", 20).into_font())
            .x_label_area_size(10.percent())
            .y_label_area_size(10.percent())
            .margin(5.percent())
            .build_cartesian_2d(
                (value_range.start as f32 * 0.85)..(value_range.end as f32 * 1.15),
                segments.into_segmented(),
            )?;

        chart.configure_mesh().disable_mesh().draw()?;

        let data = graph.graph_result.iter().fold(
            [vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]],
            |[
             mut cycles,
             mut total_cycle_ignore_empty,
             mut total_cycle_fix_empty,
             mut ignore_empty_row_meta_cycle,
             mut fix_empty_row_meta_cycle,
             mut meta_cycles,
             mut comp_cycles,
             mut row_opens,
             ],
             c| {
                cycles.push(c.cycle);
                total_cycle_ignore_empty.push(c.total_cycle_ignore_empty_meta);
                total_cycle_fix_empty.push(c.total_cycle_fix_empty_meta);
                ignore_empty_row_meta_cycle.push(c.ignore_empty_row_meta_cycle);
                fix_empty_row_meta_cycle.push(c.fix_empty_meta_cycle);
                meta_cycles.push(c.meta_cycle);
                comp_cycles.push(c.compute_cycle);
                row_opens.push(c.row_open);
                [
                    cycles,
                    total_cycle_ignore_empty,
                    total_cycle_fix_empty,
                    ignore_empty_row_meta_cycle,
                    fix_empty_row_meta_cycle,
                    meta_cycles,
                    comp_cycles,
                    row_opens,
                ]
            },
        );
        let maxes = data.iter().map(|x| x.iter().max().unwrap()).collect_vec();
        let quartiles = data
            .iter()
            .map(|x| Quartiles::new(x.iter().map(|x| *x as f32).collect_vec().as_slice()))
            .collect_vec();

        chart.draw_series(types.iter().zip(quartiles.iter()).zip(colors.iter()).map(
            |((name, data), color)| {
                Boxplot::new_horizontal(SegmentValue::CenterOf(name), data).style(color)
            },
        ))?;
        chart.draw_series(types.iter().zip(maxes.iter()).zip(colors.iter()).map(
            |((name, &&data), color)| {
                Rectangle::new(
                    [
                        (
                            data as f32 + range_size as f32 * 0.01,
                            SegmentValue::CenterOf(name),
                        ),
                        (
                            data as f32 - range_size as f32 * 0.01,
                            SegmentValue::Exact(name),
                        ),
                    ],
                    color.mix(0.5).filled(),
                )
            },
        ))?;
        chart.configure_series_labels().draw()?;
    }
    root.present()?;
    Ok(())
}
