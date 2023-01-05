use std::{error::Error, fs::File, io::BufReader};

use clap::Parser;
use itertools::Itertools;
use plotters::{
    coord::Shift,
    data::fitting_range,
    prelude::*,
    style::full_palette::{BLUEGREY, PINK},
};
use plotters_text::TextDrawingBackend;
use spmspm_pim::{
    analysis::{analyze_gearbox::GearboxReslt, analyze_split_spmm::SplitAnalyzeResult},
    cli::{DrawCli, SpeedUpArgs, SplitArgs},
    init_logger_info,
};
use tracing::info;
#[derive(Debug)]
enum Ext {
    Png,
    Svg,
    Console,
}

const MIN_CONSOLE_WIDTH: u16 = 320;
const MIN_CONSOLE_HEIGHT: u16 = 60;

type SpeedUp = (f32, f32, f32, f32);
fn draw<'a, DB: DrawingBackend + 'a>(
    root: DrawingArea<DB, Shift>,
    data: &[(String, SpeedUp)],
) -> Result<(), Box<dyn Error + 'a>> {
    let (left, right) = root.split_horizontally(root.dim_in_pixel().0 as f32 * (2.0 / 3.0));

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
        .margin(1)
        .x_label_area_size(10.percent_height())
        .y_label_area_size(10.percent_width())
        .build_cartesian_2d(0f32..1f32, 0f32..max_hight * 1.2)?;
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
        .margin(10.percent())
        .x_label_area_size(0)
        .y_label_area_size(0)
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
    root.present()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = DrawCli::parse();
    init_logger_info();
    info!("start draw");

    match args.subcmd {
        spmspm_pim::cli::DrawType::SpeedUp(speed_up_args) => draw_speedup(speed_up_args)?,
        spmspm_pim::cli::DrawType::Split(split_args) => draw_split(split_args)?,
    }
    Ok(())
}

fn draw_speedup(args: SpeedUpArgs) -> Result<(), Box<dyn Error>> {
    // get the speed up from spmm and gearbox
    // first calculate the runting for out design
    let split_path = args
        .split_result
        .unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let gearbox_path = args
        .gearbox_result
        .unwrap_or_else(|| "output/gearbox_out_001_gearbox.json".into());
    let output_path = args.output.unwrap_or_else(|| "console".into());
    let ext = match output_path.extension() {
        Some(ext) => match ext.to_str().unwrap() {
            "png" => Ext::Png,
            "svg" => Ext::Svg,
            _ => {
                let terminal_size = terminal_size::terminal_size().unwrap();
                if terminal_size.0 .0 < MIN_CONSOLE_WIDTH || terminal_size.1 .0 < MIN_CONSOLE_HEIGHT
                {
                    eprintln!(
            "terminal size is too small,current size is {}x{}, require {MIN_CONSOLE_WIDTH}x{MIN_CONSOLE_HEIGHT}",
            terminal_size.0 .0, terminal_size.1 .0
        );
                    std::process::exit(1);
                };
                Ext::Console
            }
        },
        None => {
            let terminal_size = terminal_size::terminal_size().unwrap();
            if terminal_size.0 .0 < MIN_CONSOLE_WIDTH || terminal_size.1 .0 < MIN_CONSOLE_HEIGHT {
                eprintln!(
            "terminal size is too small,current size is {}x{}, require {MIN_CONSOLE_WIDTH}x{MIN_CONSOLE_HEIGHT}",
            terminal_size.0 .0, terminal_size.1 .0
        );
                std::process::exit(1);
            };
            Ext::Console
        }
    };
    info!("ext is {:?}", ext);
    info!("start parsing {:?} and {:?}", split_path, gearbox_path);
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_path)?))?;
    info!("finish parsing split");
    let gearbox_result: GearboxReslt =
        serde_json::from_reader(BufReader::new(File::open(gearbox_path)?))?;
    info!("finish parsing gearbox");
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
    match ext {
        Ext::Png => {
            let root = BitMapBackend::new(&output_path, (1920, 1080)).into_drawing_area();
            root.fill(&WHITE)?;

            draw(root, &data).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            });
        }
        Ext::Svg => {
            let root = SVGBackend::new(&output_path, (1920, 1080)).into_drawing_area();
            root.fill(&WHITE)?;

            draw(root, &data).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            });
        }
        Ext::Console => {
            info!("draw to console");
            let terminal_size = terminal_size::terminal_size().unwrap();

            let root =
                TextDrawingBackend::new(terminal_size.0 .0 as u32, terminal_size.1 .0 as u32)
                    .into_drawing_area();
            draw(root, &data).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            });
        }
    }
    Ok(())
}

fn draw_split(args: SplitArgs) -> Result<(), Box<dyn Error>> {
    let SplitArgs {
        split_result,
        output,
    } = args;
    let output_path = output.unwrap_or_else(|| "console.svg".into());
    let split_result =
        split_result.unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
    let split_result: SplitAnalyzeResult =
        serde_json::from_reader(BufReader::new(File::open(split_result)?))?;
    // generate the box plot for each graph
    let root = SVGBackend::new(&output_path, (1920, 1080)).into_drawing_area();
    draw_box(root, &split_result).unwrap_or_else(|err| {
        eprintln!("error: {}", err);
        std::process::exit(1);
    });
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
        let segs = types.clone();
        let range_size = value_range.end - value_range.start;
        let mut chart = ChartBuilder::on(&chart)
            .caption(graph.name.clone(), ("sans-serif", 20).into_font())
            .x_label_area_size(10.percent())
            .y_label_area_size(10.percent())
            .margin(5.percent())
            .build_cartesian_2d(
                (value_range.start as f32 * 0.85)..(value_range.end as f32 * 1.15),
                segs.into_segmented(),
            )?;

        chart.configure_mesh().disable_mesh().draw()?;

        let data = graph.graph_result.iter().fold(
            [vec![], vec![], vec![], vec![], vec![], vec![], vec![],  vec![]],
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
        let maxs = data.iter().map(|x| x.iter().max().unwrap()).collect_vec();
        let quatiles = data
            .iter()
            .map(|x| Quartiles::new(x.iter().map(|x| *x as f32).collect_vec().as_slice()))
            .collect_vec();

        chart.draw_series(types.iter().zip(quatiles.iter()).zip(colors.iter()).map(
            |((name, data), color)| {
                Boxplot::new_horizontal(SegmentValue::CenterOf(name), data).style(color)
            },
        ))?;
        chart.draw_series(types.iter().zip(maxs.iter()).zip(colors.iter()).map(
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

#[cfg(test)]
mod tests {

    use sprs::{num_kinds::Pattern, CsMat};

    #[test]
    fn test_read_mtx() {
        const MTX_PATH: &str = "mtx/gearbox/ca-hollywood-2009.mtx";
        let _graph: CsMat<Pattern> = sprs::io::read_matrix_market(MTX_PATH).unwrap().to_csr();
    }
}
