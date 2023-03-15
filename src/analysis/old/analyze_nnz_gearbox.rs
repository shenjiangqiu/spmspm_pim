//! this module is used to analyze the nnz

use plotters::{prelude::*, style::AsRelative};
use std::{
    cmp::Reverse,
    fmt::{Debug, Display},
    ops::Add,
    path::Path,
};

use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat, CsVec, CsVecView};
use tracing::info;

use crate::{
    draw::{self, DrawFn},
    pim::{
        config::Config,
        level::{ddr4, LevelTrait},
    },
};

use super::split::NnzStats;

/// the statistics of a single graph
#[derive(Serialize, Deserialize)]
pub struct SingleResult {
    /// the name of the graph
    pub name: String,
    /// the nnz statistics of the graph
    pub nnz_stats: NnzStats,
}
#[derive(Serialize, Deserialize)]
/// the statistics of all graphs
pub struct NnzAnalyzeResult {
    /// the statistics of all graphs
    pub results: Vec<SingleResult>,
}

impl NnzAnalyzeResult {
    /// print out all the results
    pub fn show_results(&self) {
        for result in &self.results {
            println!("---------------------------");
            println!("\n\nname -------: {}", result.name);
            println!("nnz_stats: {:?}", result.nnz_stats);
        }
    }
}

impl Debug for NnzAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for NnzAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for result in &self.results {
            writeln!(f, "name: {}", result.name)?;
            writeln!(f, "nnz_stats: {:?}", result.nnz_stats)?;
        }
        Ok(())
    }
}

/// analyze the split spmm
pub(crate) fn analyze_nnz_spmm(config: &Config) {
    match config.dram_type {
        crate::pim::config::DramType::DDR3 => todo!(),
        crate::pim::config::DramType::DDR4 => {
            let total_size = ddr4::Storage::new(
                config.channels.num,
                config.ranks.num,
                config.chips.num,
                config.bank_groups.num,
                config.banks.num,
                config.subarrays,
                config.rows,
                config.columns,
            );
            analyze_nnz_spmm_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

fn analyze_nnz_spmm_inner<LevelType: LevelTrait>(config: &Config, _total_size: &LevelType::Storage)
where
    LevelType::Storage: Debug + Sync,
    LevelType::Mapping: Debug,
{
    let total_graphs = config.graph_path.len();
    for (index, path) in config.graph_path.iter().enumerate() {
        info!("analyzing graph {}/{}", index + 1, total_graphs);

        let matrix_a: CsMat<Pattern> = sprs::io::read_matrix_market(path)
            .unwrap_or_else(|e| {
                panic!("failed to read matrix market file:{path}; {}", e);
            })
            .to_csr();
        let mut nnzs = matrix_a
            .outer_iterator()
            .map(|v| v.nnz())
            .collect::<Vec<_>>();
        nnzs.sort_unstable_by_key(|x| Reverse(*x));
        let mut current_sum = 0;
        let mut acc = Vec::with_capacity(nnzs.len());
        for nnz in &nnzs {
            current_sum += *nnz;
            acc.push(current_sum);
        }
        // draw the graph
        draw::draw_data::<_, NnzDrawer>(
            Path::new(&format!(
                "nnz_{}.svg",
                Path::new(path).file_stem().unwrap().to_string_lossy()
            )),
            &(nnzs, acc),
        )
        .unwrap();
    }
}
struct NnzDrawer;
impl DrawFn for NnzDrawer {
    type DATA = (Vec<usize>, Vec<usize>);

    fn draw_apply<'a, DB: plotters::prelude::DrawingBackend + 'a>(
        root: plotters::prelude::DrawingArea<DB, plotters::coord::Shift>,
        data: &Self::DATA,
    ) -> Result<(), Box<dyn std::error::Error + 'a>> {
        let num_datas = data.0.len();
        let max_data = data.0.iter().max().unwrap();
        let max = data.1.last().unwrap();
        let (left, right) = root.split_horizontally(50.percent_width());
        let mut left_chart = ChartBuilder::on(&left)
            .x_label_area_size(5.percent())
            .y_label_area_size(5.percent())
            .build_cartesian_2d(0..num_datas, 0..*max_data)?;
        left_chart.configure_mesh().disable_mesh().draw()?;

        left_chart.draw_series(data.0.iter().enumerate().map(|(index, nnz)| {
            Rectangle::new([(index, 0), (index + 1, *nnz)], RED.mix(0.5).filled())
        }))?;

        let mut right_chart = ChartBuilder::on(&right)
            .x_label_area_size(5.percent())
            .y_label_area_size(5.percent())
            .build_cartesian_2d(0..num_datas, 0f32..1f32)?;
        right_chart.configure_mesh().disable_mesh().draw()?;
        let data_acc = data
            .1
            .iter()
            .map(|x| *x as f32 / *max as f32)
            .collect::<Vec<_>>();
        let ninety_percent = data_acc
            .iter()
            .enumerate()
            .find(|(_, x)| **x >= 0.9)
            .unwrap()
            .0;
        right_chart.draw_series(data.1.iter().enumerate().map(|(index, nnz)| {
            Rectangle::new(
                [(index, 0f32), (index + 1, *nnz as f32 / *max as f32)],
                RED.mix(0.5).filled(),
            )
        }))?;
        let corss_lines = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]
            .into_iter()
            .flat_map(|x| {
                [
                    LineSeries::new(
                        [
                            ((num_datas as f32 * x) as usize, 0.),
                            ((num_datas as f32 * x) as usize, 1.),
                        ],
                        BLACK,
                    ),
                    LineSeries::new(
                        [
                            (0, data_acc[(num_datas as f32 * x) as usize]),
                            (num_datas, data_acc[(num_datas as f32 * x) as usize]),
                        ],
                        BLACK,
                    ),
                ]
            })
            .flatten();
        right_chart.draw_series(
            [
                LineSeries::new([(0, 0.9), (num_datas, 0.9)], BLUE),
                LineSeries::new([(ninety_percent, 0.), (ninety_percent, 1.)], BLACK),
            ]
            .into_iter()
            .flatten()
            .chain(corss_lines),
        )?;

        Ok(())
    }
}

/// the stat result of the seq spmm
#[derive(Debug, Serialize, Deserialize)]
pub struct SeqResult {
    /// the cycles
    pub cycle: u64,
    /// the graph name
    pub name: String,
    /// compute cycles
    pub compute_cycle: u64,
    /// the time spent on row_open:
    pub row_open: u64,
    /// row open no overlap
    pub row_open_no_overlap: u64,
    /// the time spent on temp_result_read
    pub temp_result_read: u64,
    /// the time spent on final_result_write
    pub final_result_write: u64,
    /// the time spent on matrix_b_read
    pub matrix_b_read: u64,
    /// the number of bytes that row opens
    pub row_open_bytes: usize,
    /// the number of bytes that really used
    pub used_bytes: usize,
    /// total input read bytes
    pub input_read_bytes: usize,
    /// total input read times
    pub input_read_times: usize,
}

/// add two vector and return a new vector(sparse)
/// # Example
/// ```
/// use spmspm_pim::analysis::analyze_nnz_spmm;
/// use sprs::{CsVec, CsVecView};
/// let v1 = CsVec::new(5, vec![0, 2, 4], vec![1,1,1]);
/// let v2 = CsVec::new(5, vec![1, 3, 4], vec![1,1,1]);
/// let v3 = analyze_nnz_spmm::sparse_add(v1.view(), v2.view());
/// assert_eq!(v3, CsVec::new(5, vec![0, 1, 2, 3, 4], vec![1,1,1,1,2]));
/// ```
pub fn sparse_add<T>(v1: CsVecView<T>, v2: CsVecView<T>) -> CsVec<T>
where
    T: Add<Output = T> + Copy,
{
    assert_eq!(v1.dim(), v2.dim());
    let mut v1_iter = v1.iter();
    let mut v2_iter = v2.iter();
    let mut v1_next = v1_iter.next();
    let mut v2_next = v2_iter.next();
    let mut result = CsVec::empty(v1.dim());
    while v1_next.is_some() || v2_next.is_some() {
        match (v1_next, v2_next) {
            (Some((i1, v1)), Some((i2, v2))) => match i1.cmp(&i2) {
                std::cmp::Ordering::Less => {
                    result.append(i1, *v1);
                    v1_next = v1_iter.next();
                }
                std::cmp::Ordering::Equal => {
                    result.append(i1, *v1 + *v2);
                    v1_next = v1_iter.next();
                    v2_next = v2_iter.next();
                }
                std::cmp::Ordering::Greater => {
                    result.append(i2, *v2);
                    v2_next = v2_iter.next();
                }
            },
            (Some((i1, v1)), None) => {
                result.append(i1, *v1);
                v1_next = v1_iter.next();
            }
            (None, Some((i2, v2))) => {
                result.append(i2, *v2);
                v2_next = v2_iter.next();
            }
            (None, None) => unreachable!(),
        }
    }
    result
}
