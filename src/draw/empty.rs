use std::{error::Error, fs::File, io::BufReader, path::PathBuf};

use plotters::{coord::Shift, prelude::*};
use tracing::info;

use super::{draw_data, DrawFn};
use crate::{analysis::analyze_split_spmm::SplitAnalyzeResult, cli::ExecResult};

struct EmptyDrawer;

impl DrawFn for EmptyDrawer {
    type DATA = SplitAnalyzeResult;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        draw_empty_rec(root, data)
    }
}

pub fn draw_empty(args: ExecResult) -> eyre::Result<()> {
    let ExecResult {
        result_file,
        output,
    } = args;
    let output_path = output.unwrap_or_else(|| "empty.svg".into());
    let split_result =
        result_file.unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_result)?))?;
    // generate the box plot for each graph
    draw_data::<_, EmptyDrawer>(&output_path, &split_result)?;

    Ok(())
}

fn draw_empty_rec<'a, DB: DrawingBackend + 'a>(
    root: DrawingArea<DB, Shift>,
    result: &SplitAnalyzeResult,
) -> Result<(), Box<dyn Error + 'a>> {
    let charts = root.split_evenly((4, 5));
    for (graph, chart) in result.results.iter().zip(charts) {
        info!("draw graph {}", graph.name);
        // first get the min_max for the cycles for each bank

        let empty_rate: f32 = graph
            .graph_result
            .iter()
            .map(|x| {
                (x.total_empty_row as f32) / ((x.total_empty_row + x.total_non_empt_row) as f32)
            })
            .sum::<f32>()
            / graph.graph_result.len() as f32;

        let name = PathBuf::from(graph.name.clone());
        let mut chart = ChartBuilder::on(&chart)
            .caption(
                format!(
                    "{}:{}",
                    name.file_name().unwrap().to_str().unwrap(),
                    empty_rate
                ),
                ("sans-serif", 20).into_font(),
            )
            .x_label_area_size(10.percent())
            .y_label_area_size(10.percent())
            .margin(5.percent())
            .build_cartesian_2d(0f32..1f32, 0f32..1f32)?;

        chart.configure_mesh().disable_mesh().draw()?;

        chart.draw_series([
            Rectangle::new([(0f32, 0f32), (1f32, empty_rate)], BLACK.mix(0.5).filled()),
            Rectangle::new([(0f32, empty_rate), (1f32, 1f32)], RED.mix(0.5).filled()),
        ])?;

        chart.configure_series_labels().draw()?;
    }
    root.present()?;
    Ok(())
}
