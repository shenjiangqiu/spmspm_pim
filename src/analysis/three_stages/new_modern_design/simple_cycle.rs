use sprs::{num_kinds::Pattern, CsMatI};

use crate::{
    analysis::{
        mapping::Mapping,
        traits::{AnalyzeTool, GearboxSimTrait},
    },
    pim::configv2::ConfigV2,
};

pub struct SimpleCycleAnalysis {}
pub struct Simulator<'matrix, 'config, Mapping> {
    config: &'config ConfigV2,
    matrix: &'matrix CsMatI<Pattern, u32>,
    mapping: Mapping,
}
pub struct Result {}
impl<'matrix, 'config, MP: Mapping> GearboxSimTrait<'matrix, 'config>
    for Simulator<'matrix, 'config, MP>
{
    type Mapping = MP;

    type SingleResult = Result;

    fn new(
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: &'matrix CsMatI<Pattern, u32>,
        config: &'config ConfigV2,
        mapping: Self::Mapping,
    ) -> Self {
        todo!()
    }

    fn get_evil_row_ids(&self) -> usize {
        todo!()
    }

    fn get_evil_col_ids(&self) -> usize {
        todo!()
    }

    fn get_mapping(&self) -> &Self::Mapping {
        todo!()
    }

    fn evil_row_contains(&self, row_id: usize) -> bool {
        todo!()
    }

    fn evil_col_contains(&self, col_id: usize) -> bool {
        todo!()
    }

    fn get_matrix_b(&self) -> &CsMatI<Pattern, u32> {
        todo!()
    }

    fn report(&self, name: String, batch: usize, topk: f32) -> Self::SingleResult {
        todo!()
    }
}
impl AnalyzeTool for SimpleCycleAnalysis {
    type ResultType = Result;

    const SUBARRAY_SIZE: usize = 1;

    type GearboxSimType<'matrix, 'config, T: crate::analysis::mapping::Mapping> =
        Simulator<'matrix, 'config, T>;
}
