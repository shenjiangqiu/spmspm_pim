use std::{error::Error, fmt::Debug, path::Path};

use plotters::{coord::Shift, prelude::*};
use plotters_text::TextDrawingBackend;
use terminal_size::{Height, Width};
use tracing::info;
mod cycle_dist;
mod draw_split;
mod empty;
mod gearbox;
mod gearbox_all;
mod gearbox_old;
mod run;
mod speedup;
pub use run::draw_with_type;
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
fn check_terminal_size(terminal_size: (Width, Height)) {
    if terminal_size.0 .0 < MIN_CONSOLE_WIDTH || terminal_size.1 .0 < MIN_CONSOLE_HEIGHT {
        eprintln!(
            "terminal size is too small,current size is {}x{}, require {MIN_CONSOLE_WIDTH}x{MIN_CONSOLE_HEIGHT}",
            terminal_size.0.0, terminal_size.1.0
        );
        std::process::exit(1);
    };
}

pub fn get_ext(output_path: &Path) -> Ext {
    let ext = match output_path.extension() {
        Some(ext) => match ext.to_str().unwrap() {
            "png" => Ext::Png,
            "svg" => Ext::Svg,
            _ => {
                let terminal_size = terminal_size::terminal_size().unwrap();
                check_terminal_size(terminal_size);
                Ext::Console
            }
        },
        None => {
            let terminal_size = terminal_size::terminal_size().unwrap();
            check_terminal_size(terminal_size);
            Ext::Console
        }
    };
    ext
}

/// the generic fn to draw the data using the DrawFn
pub fn draw_data<DATA: ?Sized, F: DrawFn<DATA = DATA>>(
    output_path: &Path,
    split_result: &DATA,
) -> eyre::Result<()> {
    info!("draw data into {:?}", output_path);
    match get_ext(output_path) {
        Ext::Svg => {
            let root = SVGBackend::new(&output_path, (1920, 1080)).into_drawing_area();
            root.fill(&WHITE)?;
            F::draw_apply(root, split_result).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            })
        }
        Ext::Png => {
            let root = BitMapBackend::new(&output_path, (1920, 1080)).into_drawing_area();
            info!("draw png");
            root.fill(&WHITE)?;
            F::draw_apply(root, split_result).unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            })
        }
        Ext::Console => {
            let terminal_size = terminal_size::terminal_size().unwrap();

            F::draw_apply(
                TextDrawingBackend::new(terminal_size.0 .0 as u32, terminal_size.1 .0 as u32)
                    .into_drawing_area(),
                split_result,
            )
            .unwrap_or_else(|err| {
                eprintln!("error: {}", err);
                std::process::exit(1);
            })
        }
    };
    Ok(())
}
