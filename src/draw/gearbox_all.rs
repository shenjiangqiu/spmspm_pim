use std::{
    error::Error,
    fs::File,
    io::BufReader,
    path::{self},
};

use eyre::Context;
use plotters::{coord::Shift, prelude::*};
use tracing::info;

use super::{draw_data, DrawFn};
use crate::{analysis::analyze_gearbox_origin_all, cli::ExecResult};

/// draw gearbox cycle distribution
pub fn draw_gearbox_all(gearbox_args: ExecResult) -> eyre::Result<()> {
    let ExecResult {
        result_file,
        output,
    } = gearbox_args;
    let output_path = output.unwrap_or_else(|| "console.svg".into());
    let gearbox_result =
        result_file.unwrap_or_else(|| "output/gearbox_g_gearbox_origin.json".into());
    let gearbox_result: analyze_gearbox_origin_all::GearboxResult =
        serde_json::from_reader(BufReader::new(File::open(gearbox_result)?))
            .wrap_err(eyre::eyre!("cannot parse the file to gearbox result"))?;
    // generate the box plot for each graph
    draw_data::<_, GearboxAllDrawer>(&output_path, &gearbox_result)?;

    Ok(())
}

struct GearboxAllDrawer;
impl DrawFn for GearboxAllDrawer {
    type DATA = analyze_gearbox_origin_all::GearboxResult;

    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let charts = root.split_evenly((2, 3));
        for (graph, chart) in data.results.iter().zip(charts.iter()) {
            info!("draw graph {}", graph.name);
            // first get the min_max for the cycles for each bank
            // the total max
            let &analyze_gearbox_origin_all::TotalResult {
                global_max_acc_cycle,
                global_max_acc_cycle_remote,
                gloabl_max_acc_ring,
                global_max_acc_tsv,
            } = &graph.total_result;

            let local_acc_cycle_max = graph.subarray_result.iter().map(|x| x.cycle).max().unwrap();

            let remote_acc_cycle_max = graph
                .subarray_result
                .iter()
                .map(|x| x.cycle_remote)
                .max()
                .unwrap();
            let ring_cycle_max = graph.ring_result.iter().map(|x| x.cycle).max().unwrap();
            let tsv_cycle_max = graph.tsv_result.iter().map(|x| x.cycle).max().unwrap();

            let &max = [
                local_acc_cycle_max,
                ring_cycle_max,
                tsv_cycle_max,
                remote_acc_cycle_max,
                global_max_acc_cycle,
                global_max_acc_cycle_remote,
                gloabl_max_acc_ring,
                global_max_acc_tsv,
            ]
            .iter()
            .max()
            .unwrap();
            let name = path::Path::new(&graph.name);
            let mut chart = ChartBuilder::on(chart)
                .caption(
                    name.file_stem().unwrap().to_str().unwrap(),
                    ("sans-serif", 20).into_font(),
                )
                .x_label_area_size(10.percent())
                .y_label_area_size(10.percent())
                .margin(5.percent())
                .build_cartesian_2d(0usize..4, 0..max)?;
            let data = [
                (global_max_acc_cycle, local_acc_cycle_max),
                (gloabl_max_acc_ring, ring_cycle_max),
                (global_max_acc_tsv, tsv_cycle_max),
                (global_max_acc_cycle_remote, remote_acc_cycle_max),
            ];
            // this is the real gearbox runtime cycle
            let final_cycle = global_max_acc_cycle
                + global_max_acc_cycle_remote
                + gloabl_max_acc_ring
                + global_max_acc_tsv;
            let pipe_line_cycle = [
                local_acc_cycle_max,
                ring_cycle_max,
                tsv_cycle_max,
                remote_acc_cycle_max,
            ]
            .into_iter()
            .max()
            .unwrap();
            let max_possible_speedup = final_cycle as f64 / pipe_line_cycle as f64;

            chart.configure_mesh().disable_mesh().draw()?;
            chart.draw_series(data.into_iter().enumerate().flat_map(
                |(index, (global_max, acc_max))| {
                    [
                        Rectangle::new(
                            [(index, 0), (index + 1, global_max)],
                            BLACK.mix(0.5).filled(),
                        ),
                        Rectangle::new([(index, 0), (index + 1, acc_max)], RED.mix(0.5).filled()),
                    ]
                },
            ))?;
            chart
                .draw_series(["local", "ring", "tsv", "remote"].iter().enumerate().map(
                    |(index, &name)| Text::new(name, (index, 0), ("sans-serif", 20).into_font()),
                ))
                .unwrap();
            chart
                .draw_series([Text::new(
                    format!("max possible speedup: {:.2}", max_possible_speedup),
                    (4, max),
                    ("sans-serif", 20).into_font(),
                )])
                .unwrap();
            chart.configure_series_labels().draw()?;
        }

        // for (graph, chart) in data
        //     .results
        //     .iter()
        //     .zip(charts.iter().skip(data.results.len()))
        // {
        //     info!("draw graph {}", graph.name);
        //     // first get the min_max for the cycles for each bank
        //     let local_acc_cycle_max = graph.subarray_result.iter().map(|x| x.cycle).max().unwrap();
        //     let local_acc_cycle_mean = graph.subarray_result.iter().map(|x| x.cycle).sum::<usize>()
        //         / graph.subarray_result.len();

        //     let ring_cycle_max = graph.ring_result.iter().map(|x| x.cycle).max().unwrap();
        //     let ring_cycle_mean =
        //         graph.ring_result.iter().map(|x| x.cycle).sum::<usize>() / graph.ring_result.len();

        //     let tsv_cycle_max = graph.tsv_result.iter().map(|x| x.cycle).max().unwrap();
        //     let tsv_cycle_mean =
        //         graph.tsv_result.iter().map(|x| x.cycle).sum::<usize>() / graph.tsv_result.len();

        //     let &max_mean = [local_acc_cycle_mean, ring_cycle_mean, tsv_cycle_mean]
        //         .iter()
        //         .max()
        //         .unwrap();

        //     let name = Path::new(&graph.name);
        //     let mut chart = ChartBuilder::on(&chart)
        //         .caption(
        //             name.file_stem().unwrap().to_str().unwrap(),
        //             ("sans-serif", 20).into_font(),
        //         )
        //         .x_label_area_size(10.percent())
        //         .y_label_area_size(10.percent())
        //         .margin(5.percent())
        //         .build_cartesian_2d(0usize..4, 0..max_mean)?;
        //     let data = [
        //         (local_acc_cycle_max, local_acc_cycle_mean),
        //         (ring_cycle_max, ring_cycle_mean),
        //         (tsv_cycle_max, tsv_cycle_mean),
        //     ];
        //     chart.configure_mesh().disable_mesh().draw()?;
        //     chart.draw_series(data.into_iter().enumerate().map(|(index, (_max, mean))| {
        //         Rectangle::new([(index, 0), (index + 1, mean)], BLUE.mix(0.5).filled())
        //     }))?;
        //     chart.configure_series_labels().draw()?;
        // }

        root.present()?;
        Ok(())
    }
}
