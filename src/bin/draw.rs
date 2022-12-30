use std::{fs::File, io::BufReader};

use clap::Parser;
use plotters::prelude::*;
use spmspm_pim::{
    analysis::{analyze_gearbox::GearboxReslt, analyze_split_spmm::SplitAnalyzeResult},
    cli::DrawCli,
    init_logger_info,
};

fn main() -> eyre::Result<()> {
    init_logger_info();
    let args = DrawCli::parse();
    // get the speed up from spmm and gearbox
    // first calculate the runting for out design
    let split_path = args
        .split_result
        .unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let gearbox_path = args
        .gearbox_result
        .unwrap_or_else(|| "output/gearbox_out_001_gearbox.json".into());
    let output_path = args
        .output
        .unwrap_or_else(|| "output/gearbox_out_001.png".into());
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_path)?))?;
    let gearbox_result: GearboxReslt =
        serde_json::from_reader(BufReader::new(File::open(gearbox_path)?))?;
    let mut data = vec![];
    for (split, gearbox) in split_result.results.into_iter().zip(gearbox_result.results) {
        assert_eq!(split.name, gearbox.name);
        // first get the runtime
        let split_time = split
            .graph_result
            .iter()
            .map(|x| x.cycle)
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
        data.push((
            split.name,
            (speed_up, speed_up_tsv, speed_up_ring, speed_up_comp),
        ));
    }
    // draw the speed up using plotlib
    let root = BitMapBackend::new(&output_path, (1920, 1080)).into_drawing_area();
    let (left, right) = root.split_horizontally(1200);
    root.fill(&WHITE)?;

    let num_recs = data.len();
    let gap = 1.0 / num_recs as f32;
    let width = gap * 0.8;
    let max_hight = data
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
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..1f32, 0f32..max_hight * 1.2)?;
    chart.configure_mesh().draw()?;

    chart.draw_series(data.iter().enumerate().map(|(id, (_, speedup))| {
        Rectangle::new(
            [
                (id as f32 * gap, 0f32),
                (id as f32 * gap + width, speedup.0 as f32),
            ],
            RED.filled(),
        )
    }))?;
    // draw a yellow line for tsv speedup
    for (id, (_, speedup)) in data.iter().enumerate() {
        chart.draw_series(LineSeries::new(
            [
                ((id as f32 * gap), speedup.1 as f32),
                (id as f32 * gap + width * 1.2, speedup.1 as f32),
            ],
            BLACK,
        ))?;
    }
    // draw a green line for ring speedup
    for (id, (_, speedup)) in data.iter().enumerate() {
        chart.draw_series(LineSeries::new(
            [
                ((id as f32 * gap), speedup.2 as f32),
                (id as f32 * gap + width * 1.2, speedup.2 as f32),
            ],
            GREEN,
        ))?;
    }
    // draw a blue line for comp speedup
    for (id, (_, speedup)) in data.iter().enumerate() {
        chart.draw_series(LineSeries::new(
            [
                ((id as f32 * gap), speedup.3 as f32),
                (id as f32 * gap + width * 1.2, speedup.3 as f32),
            ],
            BLUE,
        ))?;
    }

    // draw a line at y=1
    chart.draw_series(LineSeries::new(vec![(0f32, 1f32), (1f32, 1f32)], BLACK))?;
    // draw the names
    let mut right_chart = ChartBuilder::on(&right)
        .caption("Names", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..1f32, 0f32..1f32)?;
    // right_chart.configure_mesh().disable_mesh().draw()?;

    right_chart.draw_series(data.iter().enumerate().map(|(id, (name, speedup))| {
        let name = std::path::Path::new(name);
        let file_name = name.file_name().unwrap().to_str().unwrap();
        Text::new(
            format!("{}-{}-{:?}", id, file_name, speedup),
            (0., (id as f32 * gap)),
            ("sans-serif", 20).into_font(),
        )
    }))?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use sprs::{num_kinds::Pattern, CsMat};

    #[test]
    fn test_read_mtx() {
        const MTX_PATH: &str = "mtx/gearbox/ca-hollywood-2009.mtx";
        let _graph: CsMat<Pattern> = sprs::io::read_matrix_market(MTX_PATH).unwrap().to_csr();
    }
}
