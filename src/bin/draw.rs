use std::{
    error::Error,
    fmt::Debug,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use clap::Parser;
use itertools::Itertools;
use plotters::{
    coord::Shift,
    data::fitting_range,
    prelude::*,
    style::full_palette::{BLUEGREY, GREY, PINK},
};
use plotters_text::TextDrawingBackend;
use spmspm_pim::{
    analysis::{analyze_gearbox::GearboxResult, analyze_split_spmm::SplitAnalyzeResult},
    cli::{DrawCli, SpeedUpArgs, SplitArgs},
    init_logger_info,
};
use terminal_size::{Height, Width};
use tracing::info;
use spmspm_pim::draw::{draw_data, DrawFn, Ext, get_ext, MIN_CONSOLE_HEIGHT, MIN_CONSOLE_WIDTH};

type SpeedUp = (f32, f32, f32, f32);

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
            a.1.0
                .partial_cmp(&b.1.0)
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
        let name = Path::new(name);
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
        spmspm_pim::cli::DrawType::Empty(split_args) => draw_empty(split_args)?,
        spmspm_pim::cli::DrawType::Cycle(split_args) => draw_cycle_dist(split_args)?,
    }
    Ok(())
}

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
fn draw_cycle_dist(args: SplitArgs) -> Result<(), Box<dyn Error>> {
    let SplitArgs {
        split_result,
        output,
    } = args;
    let output_path = output.unwrap_or_else(|| "output/cycle_dist.svg".into());
    let split_result =
        split_result.unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
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
                format!("{}", name.file_name().unwrap().to_str().unwrap()),
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

fn draw_empty(args: SplitArgs) -> Result<(), Box<dyn Error>> {
    let SplitArgs {
        split_result,
        output,
    } = args;
    let output_path = output.unwrap_or_else(|| "empty.svg".into());
    let split_result =
        split_result.unwrap_or_else(|| "output/gearbox_out_001_split_spmm.json".into());
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
            .build_cartesian_2d((0f32)..(1f32), (0f32)..(1f32))?;

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

fn draw_speedup(args: SpeedUpArgs) -> Result<(), Box<dyn Error>> {
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
    let gearbox_result: GearboxResult =
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
    // draw the speed up using plotters
    draw_data::<_, SpeedUpDrawer>(&output_path, &data)?;

    Ok(())
}


fn check_terminal_size(terminal_size: (Width, Height)) {
    if terminal_size.0.0 < MIN_CONSOLE_WIDTH || terminal_size.1.0 < MIN_CONSOLE_HEIGHT {
        eprintln!(
            "terminal size is too small,current size is {}x{}, require {MIN_CONSOLE_WIDTH}x{MIN_CONSOLE_HEIGHT}",
            terminal_size.0.0, terminal_size.1.0
        );
        std::process::exit(1);
    };
}

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use spmspm_pim::cli::SplitArgs;
    use sprs::{num_kinds::Pattern, CsMat};

    use crate::draw_cycle_dist;

    #[test]
    fn test_read_mtx() {
        const MTX_PATH: &str = "mtx/gearbox/ca-hollywood-2009.mtx";
        let _graph: CsMat<Pattern> = sprs::io::read_matrix_market(MTX_PATH).unwrap().to_csr();
    }

    #[test]
    fn test_draw_cycle_png() {
        draw_cycle_dist(SplitArgs {
            split_result: None,
            output: Some(PathBuf::from("test.png")),
        })
            .unwrap();
    }
}
