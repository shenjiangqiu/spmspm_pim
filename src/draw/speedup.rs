use std::{error::Error, fs::File, io::BufReader};

use plotters::{coord::Shift, prelude::*};
use tracing::info;

use crate::{
    analysis::analyze_split_spmm::SplitAnalyzeResult,
    cli::SpeedUpArgs,
    draw::{draw_data, get_ext},
};

use super::DrawFn;
type SpeedUp = (f32, f32, f32, f32, f32, f32, f32);
use std::path;
struct SpeedUpDrawer;

impl DrawFn for SpeedUpDrawer {
    type DATA = [(String, SpeedUp)];
    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>> {
        draw(root, data)
    }
}

/// draw speedup
fn draw<'a, DB: DrawingBackend + 'a>(
    root: DrawingArea<DB, Shift>,
    data: &[(String, SpeedUp)],
) -> Result<(), Box<dyn Error + 'a>> {
    let (left, right) = root.split_horizontally(root.dim_in_pixel().0 as f32 * (2.0 / 3.0));

    let num_recs = data.len();
    let gap = 1.0 / num_recs as f32;
    let width = gap * 0.8;
    let max_height = data
        .iter()
        .max_by(|a, b| {
            a.1 .0
                .partial_cmp(&b.1 .0)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap()
        .1
         .0;
    let mut chart = ChartBuilder::on(&left)
        .caption("Speed Up", ("sans-serif", 50).into_font())
        .margin(1)
        .x_label_area_size(10.percent_height())
        .y_label_area_size(10.percent_width())
        .build_cartesian_2d(0f32..1f32, 0f32..max_height * 1.2)?;
    chart.configure_mesh().disable_mesh().draw()?;

    chart.draw_series(data.iter().enumerate().map(|(id, (_, speedup))| {
        Rectangle::new(
            [
                (id as f32 * gap, 0f32),
                (id as f32 * gap + width, speedup.0 as f32),
            ],
            RED.filled(),
        )
    }))?;
    // draw a yellow rec for tsv speedup
    for (id, (_, speedup)) in data.iter().enumerate() {
        chart.draw_series([Rectangle::new(
            [
                ((id as f32 * gap - 0.01), speedup.1 as f32),
                (
                    id as f32 * gap + width * 1.2,
                    speedup.1 as f32 + 0.01 * max_height,
                ),
            ],
            YELLOW.mix(0.5).filled(),
        )])?;
    }
    // draw a green line for ring speedup
    // for (id, (_, speedup)) in data.iter().enumerate() {
    //     chart.draw_series(LineSeries::new(
    //         [
    //             ((id as f32 * gap), speedup.2 as f32),
    //             (id as f32 * gap + width * 1.2, speedup.2 as f32),
    //         ],
    //         GREEN,
    //     ))?;
    // }
    // draw a green rec for ring speedup
    for (id, (_, speedup)) in data.iter().enumerate() {
        chart.draw_series([Rectangle::new(
            [
                ((id as f32 * gap - 0.01), speedup.2 as f32),
                (
                    id as f32 * gap + width * 1.2,
                    speedup.2 as f32 + 0.01 * max_height,
                ),
            ],
            GREEN.mix(0.5).filled(),
        )])?;
    }

    // draw a blue line for comp speedup
    // for (id, (_, speedup)) in data.iter().enumerate() {
    //     chart.draw_series(LineSeries::new(
    //         [
    //             ((id as f32 * gap), speedup.3 as f32),
    //             (id as f32 * gap + width * 1.2, speedup.3 as f32),
    //         ],
    //         BLUE,
    //     ))?;
    // }

    // draw a blue rec for comp speedup
    for (id, (_, speedup)) in data.iter().enumerate() {
        chart.draw_series([Rectangle::new(
            [
                ((id as f32 * gap - 0.01), speedup.3 as f32),
                (
                    id as f32 * gap + width * 1.2,
                    speedup.3 as f32 + 0.01 * max_height,
                ),
            ],
            BLUE.mix(0.5).filled(),
        )])?;
    }

    // draw a line at y=1
    chart.draw_series(LineSeries::new(vec![(0f32, 1f32), (1f32, 1f32)], BLACK))?;
    // draw the names
    let mut right_chart = ChartBuilder::on(&right)
        .caption("Names", ("sans-serif", 50).into_font())
        .margin(10.percent())
        .x_label_area_size(0)
        .y_label_area_size(0)
        .build_cartesian_2d(0f32..1f32, 0f32..1f32)?;
    // right_chart.configure_mesh().disable_mesh().draw()?;

    right_chart.draw_series(data.iter().enumerate().map(|(id, (name, speedup))| {
        let name = path::Path::new(name);
        let file_name = name.file_name().unwrap().to_str().unwrap();
        Text::new(
            format!("{}-{}-{:?}", id, file_name, speedup),
            (0., (id as f32 * gap)),
            ("sans-serif", 20).into_font(),
        )
    }))?;
    root.present()?;
    Ok(())
}

pub fn draw_speedup(args: SpeedUpArgs) -> eyre::Result<()> {
    // get the speed up from spmm and gearbox
    let split_path = args
        .split_result
        .unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let gearbox_path = args
        .gearbox_result
        .unwrap_or_else(|| "output/gearbox_out_001_gearbox.json".into());
    let output_path = args.output.unwrap_or_else(|| "console".into());
    let ext = get_ext(&output_path);
    info!("ext is {:?}", ext);
    info!("start parsing {:?} and {:?}", split_path, gearbox_path);
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_path)?))?;
    info!("finish parsing split");
    let gearbox_result: crate::analysis::analyze_gearbox::GearboxResult =
        serde_json::from_reader(BufReader::new(File::open(gearbox_path)?))?;
    info!("finish parsing gearbox");
    let mut data = vec![];
    for (split, gearbox) in split_result
        .results
        .into_iter()
        .zip(gearbox_result.results)
        .filter(|(_split, gearbox)| {
            // at least one cycle is larger than 10000
            // filter out the small graphs
            gearbox.tsv_result[0].cycle >= 10000
                || gearbox.ring_result[0].cycle >= 10000
                || gearbox.subarray_result[0].cycle >= 10000
        })
    {
        assert_eq!(split.name, gearbox.name);
        // first get the runtime
        let split_time = split
            .graph_result
            .iter()
            .map(|x| x.total_cycle_ignore_meta)
            .max()
            .ok_or(eyre::format_err!("no max"))?;
        let gearbox_time_ring = gearbox
            .ring_result
            .iter()
            .map(|x| x.cycle)
            .max()
            .ok_or(eyre::format_err!("no max"))?;
        let gearbox_time_tsv = gearbox
            .tsv_result
            .iter()
            .map(|x| x.cycle)
            .max()
            .ok_or(eyre::format_err!("no max"))?;
        let gearbox_time_comp = gearbox
            .subarray_result
            .iter()
            .map(|x| x.cycle * 10)
            .max()
            .ok_or(eyre::format_err!("no max"))?;
        let gearbox_time = gearbox_time_ring
            .max(gearbox_time_tsv)
            .max(gearbox_time_comp);
        let speed_up = gearbox_time as f32 / split_time as f32;
        let speed_up_tsv = gearbox_time_tsv as f32 / split_time as f32;
        let speed_up_ring = gearbox_time_ring as f32 / split_time as f32;
        let speed_up_comp = gearbox_time_comp as f32 / split_time as f32;

        let speed_up_fix_empty_meta = gearbox_time as f32
            / split
                .graph_result
                .iter()
                .map(|x| x.total_cycle_fix_empty_meta)
                .max()
                .ok_or(eyre::format_err!("no max"))? as f32;
        let speed_up_zero_empty_meta = gearbox_time as f32
            / split
                .graph_result
                .iter()
                .map(|x| x.total_cycle_ignore_empty_meta)
                .max()
                .ok_or(eyre::format_err!("no max"))? as f32;
        let speed_up_no_meta = gearbox_time as f32
            / split
                .graph_result
                .iter()
                .map(|x| x.total_cycle_ignore_meta)
                .max()
                .ok_or(eyre::format_err!("no max"))? as f32;
        data.push((
            split.name,
            (
                speed_up,
                speed_up_tsv,
                speed_up_ring,
                speed_up_comp,
                speed_up_fix_empty_meta,
                speed_up_zero_empty_meta,
                speed_up_no_meta,
            ),
        ));
    }
    // draw the speed up using plotters
    draw_data::<_, SpeedUpDrawer>(&output_path, &data)?;

    Ok(())
}
