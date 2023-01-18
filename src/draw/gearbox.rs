use std::{error::Error, fs::File, io::BufReader, path::Path};

use plotters::{coord::Shift, prelude::*};
use tracing::info;

use crate::{analysis::analyze_gearbox_origin, cli::ExecResult};

use super::{draw_data, DrawFn};

/// draw gearbox cycle distribution
pub fn draw_gearbox(gearbox_args: ExecResult) -> eyre::Result<()> {
    let ExecResult {
        result_file,
        output,
    } = gearbox_args;
    let output_path = output.unwrap_or_else(|| "console.svg".into());
    let split_result = result_file.unwrap_or_else(|| "output/gearbox_g_gearbox_origin.json".into());
    let split_result: analyze_gearbox_origin::GearboxResult =
        serde_json::from_reader(BufReader::new(File::open(split_result)?))?;
    // generate the box plot for each graph
    draw_data::<_, GearboxDrawer>(&output_path, &split_result)?;

    Ok(())
}
struct GearboxDrawer;
impl DrawFn for GearboxDrawer {
    type DATA = analyze_gearbox_origin::GearboxResult;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let charts = root.split_evenly((2, 5));
        for (graph, chart) in data.results.iter().zip(charts.iter()) {
            info!("draw graph {}", graph.name);
            // first get the min_max for the cycles for each bank
            let local_acc_cycle_max = graph.subarray_result.iter().map(|x| x.cycle).max().unwrap();
            let local_acc_cycle_mean = graph.subarray_result.iter().map(|x| x.cycle).sum::<usize>()
                / graph.subarray_result.len();

            let ring_cycle_max = graph.ring_result.iter().map(|x| x.cycle).max().unwrap();
            let ring_cycle_mean =
                graph.ring_result.iter().map(|x| x.cycle).sum::<usize>() / graph.ring_result.len();

            let tsv_cycle_max = graph.tsv_result.iter().map(|x| x.cycle).max().unwrap();
            let tsv_cycle_mean =
                graph.tsv_result.iter().map(|x| x.cycle).sum::<usize>() / graph.tsv_result.len();

            let remote_acc_cycle_max = graph
                .subarray_result
                .iter()
                .map(|x| x.cycle_remote)
                .max()
                .unwrap();
            let remote_acc_cycle_mean = graph
                .subarray_result
                .iter()
                .map(|x| x.cycle_remote)
                .sum::<usize>()
                / graph.subarray_result.len();
            let &max = [
                local_acc_cycle_max,
                ring_cycle_max,
                tsv_cycle_max,
                remote_acc_cycle_max,
            ]
            .iter()
            .max()
            .unwrap();

            let name = Path::new(&graph.name);
            let mut chart = ChartBuilder::on(&chart)
                .caption(
                    name.file_stem().unwrap().to_str().unwrap(),
                    ("sans-serif", 20).into_font(),
                )
                .x_label_area_size(10.percent())
                .y_label_area_size(10.percent())
                .margin(5.percent())
                .build_cartesian_2d(0usize..4, 0..max)?;
            let data = [
                (local_acc_cycle_max, local_acc_cycle_mean),
                (ring_cycle_max, ring_cycle_mean),
                (tsv_cycle_max, tsv_cycle_mean),
                (remote_acc_cycle_max, remote_acc_cycle_mean),
            ];
            chart.configure_mesh().disable_mesh().draw()?;
            chart.draw_series(data.into_iter().enumerate().map(|(index, (max, _mean))| {
                Rectangle::new([(index, 0), (index + 1, max)], BLACK.mix(0.5).filled())
            }))?;
            chart.configure_series_labels().draw()?;
        }
        for (graph, chart) in data
            .results
            .iter()
            .zip(charts.iter().skip(data.results.len()))
        {
            info!("draw graph {}", graph.name);
            // first get the min_max for the cycles for each bank
            let local_acc_cycle_max = graph.subarray_result.iter().map(|x| x.cycle).max().unwrap();
            let local_acc_cycle_mean = graph.subarray_result.iter().map(|x| x.cycle).sum::<usize>()
                / graph.subarray_result.len();

            let ring_cycle_max = graph.ring_result.iter().map(|x| x.cycle).max().unwrap();
            let ring_cycle_mean =
                graph.ring_result.iter().map(|x| x.cycle).sum::<usize>() / graph.ring_result.len();

            let tsv_cycle_max = graph.tsv_result.iter().map(|x| x.cycle).max().unwrap();
            let tsv_cycle_mean =
                graph.tsv_result.iter().map(|x| x.cycle).sum::<usize>() / graph.tsv_result.len();

            let remote_acc_cycle_max = graph
                .subarray_result
                .iter()
                .map(|x| x.cycle_remote)
                .max()
                .unwrap();
            let remote_acc_cycle_mean = graph
                .subarray_result
                .iter()
                .map(|x| x.cycle_remote)
                .sum::<usize>()
                / graph.subarray_result.len();
            let &max_mean = [
                local_acc_cycle_mean,
                ring_cycle_mean,
                tsv_cycle_mean,
                remote_acc_cycle_mean,
            ]
            .iter()
            .max()
            .unwrap();

            let name = Path::new(&graph.name);
            let mut chart = ChartBuilder::on(&chart)
                .caption(
                    name.file_stem().unwrap().to_str().unwrap(),
                    ("sans-serif", 20).into_font(),
                )
                .x_label_area_size(10.percent())
                .y_label_area_size(10.percent())
                .margin(5.percent())
                .build_cartesian_2d(0usize..4, 0..max_mean)?;
            let data = [
                (local_acc_cycle_max, local_acc_cycle_mean),
                (ring_cycle_max, ring_cycle_mean),
                (tsv_cycle_max, tsv_cycle_mean),
                (remote_acc_cycle_max, remote_acc_cycle_mean),
            ];
            chart.configure_mesh().disable_mesh().draw()?;
            chart.draw_series(data.into_iter().enumerate().map(|(index, (_max, mean))| {
                Rectangle::new([(index, 0), (index + 1, mean)], BLUE.mix(0.5).filled())
            }))?;
            chart.configure_series_labels().draw()?;
        }

        root.present()?;
        Ok(())
    }
}
