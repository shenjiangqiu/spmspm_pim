#![allow(unused)]

pub mod show_global_dist;
use std::{error::Error, fmt::Debug, path::Path};

use plotters::{coord::Shift, prelude::*};
use tracing::info;
mod channel;
mod cycle_dist;
mod draw_overflow;
mod draw_refined;
mod draw_refined_dispaching;
// mod show_average_max;
// mod draw_refined_hybrid;
// mod draw_refined_distribution;
mod draw_split;
mod draw_tsv_traffic;
mod draw_v2;
mod empty;
mod gearbox;
mod gearbox_all;
mod gearbox_all_multiconf;
mod gearbox_old;
pub mod refined;
mod show_cycle;
mod speedup;
mod tsv_and_overflow;
#[derive(Debug)]
pub enum Ext {
    Png,
    Svg,
    Console,
}

pub const MIN_CONSOLE_WIDTH: u16 = 320;
pub const MIN_CONSOLE_HEIGHT: u16 = 60;

pub trait DrawFn {
    type DATA: ?Sized;
    fn draw_apply<'a, DB: DrawingBackend + 'a>(
        root: DrawingArea<DB, Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn Error + 'a>>;
}

pub fn get_ext(output_path: &Path) -> Ext {
    let ext = match output_path.extension() {
        Some(ext) => match ext.to_str().unwrap() {
            "png" => Ext::Png,
            "svg" => Ext::Svg,
            _ => {
                panic!("unsupported file extension: {}", ext.to_str().unwrap());
            }
        },
        None => {
            panic!("no file extension");
        }
    };
    ext
}
pub fn draw_data<DATA: ?Sized, F: DrawFn<DATA = DATA>>(
    output_path: &Path,
    split_result: &DATA,
) -> eyre::Result<()> {
    draw_data_with_size::<DATA, F>(output_path, split_result, (1920, 1080))
}
/// the generic fn to draw the data using the DrawFn
pub fn draw_data_with_size<DATA: ?Sized, F: DrawFn<DATA = DATA>>(
    output_path: &Path,
    split_result: &DATA,
    size: (u32, u32),
) -> eyre::Result<()> {
    info!("draw data into {:?}", output_path);
    match get_ext(output_path) {
        Ext::Svg => {
            let root = SVGBackend::new(&output_path, (size.0, size.0)).into_drawing_area();
            root.fill(&WHITE)?;
            F::draw_apply(root, split_result).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            })
        }
        Ext::Png => {
            let root = BitMapBackend::new(&output_path, (size.0, size.0)).into_drawing_area();
            info!("draw png");
            root.fill(&WHITE)?;
            F::draw_apply(root, split_result).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            })
        }
        Ext::Console => {
            unimplemented!();
        }
    };
    Ok(())
}
use crate::cli::DrawType;

pub fn draw_with_type(args: DrawType) -> eyre::Result<()> {
    match args {
        DrawType::SpeedUp(speed_up_args) => speedup::draw_speedup(speed_up_args)?,
        DrawType::Split(split_args) => draw_split::draw_split(split_args)?,
        DrawType::Empty(split_args) => empty::draw_empty(split_args)?,
        DrawType::Cycle(split_args) => cycle_dist::draw_cycle_dist(split_args)?,
        DrawType::Gearbox(gearbox_result) => gearbox::draw_gearbox(gearbox_result)?,
        DrawType::GearboxOld(gearbox_result) => gearbox_old::draw_gearbox_old(gearbox_result)?,
        DrawType::GearBoxAll(gearbox_result) => gearbox_all::draw_gearbox_all(gearbox_result)?,
        DrawType::GearBoxAllMultiConf(gearbox_result) => {
            gearbox_all_multiconf::draw_gearbox_all(gearbox_result)?
        }
        DrawType::GearBoxV2(gearbox_result) => draw_v2::draw(gearbox_result)?,
        DrawType::GearBoxOverflow(gearbox_result) => draw_overflow::draw(gearbox_result)?,
        DrawType::GearBoxTsvTraffic(gearbox_result) => draw_tsv_traffic::draw(gearbox_result)?,
        DrawType::TsvAndOverflow(gearbox_result) => tsv_and_overflow::draw(gearbox_result)?,
        DrawType::Channel(gearbox_result) => channel::draw(gearbox_result)?,
        DrawType::Refined(gearbox_result) => draw_refined::draw(gearbox_result)?,
        DrawType::RefinedDispatchOverflow(gearbox_result) => {
            draw_refined_dispaching::draw(gearbox_result)?
        }
        DrawType::RefinedHybrid(_hybrid_result) => todo!(),
        DrawType::RefinedDistribution(gearbox_result) => {
            todo!()
        }
        DrawType::ShowCycle(gearbox_result) => show_cycle::draw(gearbox_result)?,
        DrawType::ShowAvgMax(_gearbox_result) => todo!(),
        DrawType::ShowDetailedAvgMax(gearbox_result) => show_global_dist::draw(gearbox_result)?,
        DrawType::DrawTrace(_gearbox_result) => todo!(),
        DrawType::DrawTraceSplit(gearbox_result) => {
            todo!()
        }
        DrawType::DrawTraceSplitAll(gearbox_result) => {
            refined::draw_new_mapping_cycle::draw(gearbox_result)?
        }
        DrawType::DrawSimpleCycle(gearbox_result) => {
            refined::draw_new_mapping_cycle::draw(gearbox_result)?
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use sprs::{io, num_kinds::Pattern, CsMat};

    // #[test]
    // fn test_read_mtx() {
    //     const MTX_PATH: &str = "mtx/gearbox/ca-hollywood-2009.mtx";
    //     let _graph: CsMat<Pattern> = io::read_matrix_market(MTX_PATH).unwrap().to_csr();
    // }

    fn t1<'a: 'b, 'b, T>(a: &'a T, _b: &'a T) -> &'b T {
        a
    }
    #[test]
    fn test_fn() {
        let a = "123".to_string();
        let b = "234".to_string();
        let c = t1(&a, &b);

        println!("c:{}", c);
    }
}
