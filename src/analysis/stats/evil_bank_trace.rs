#![allow(unused)]

use std::collections::BTreeSet;

use crate::analysis::remap_analyze::row_cycle::*;
use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMatI};

use crate::{
    analysis::{
        mapping::Mapping,
        traits::{AnalyzeTool, DistributeType, GearboxSimTrait},
    },
    pim::configv2::ConfigV2,
};
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SingleResult;

pub struct AnalyzeBankTrace;
pub struct Simulator<'m, 'c, T> {
    mapping: T,
    result: SingleResult,
    matrix_b: &'m CsMatI<Pattern, u32>,
    config: &'c ConfigV2,
    evil_col_ids: BTreeSet<usize>,
    evil_row_ids: BTreeSet<usize>,
}

impl AnalyzeTool for AnalyzeBankTrace {
    type ResultType = SingleResult;

    type GearboxSimType<'m,'c,T> = Simulator<'m,'c,T> where T: Mapping;

    const SUBARRAY_SIZE: usize = 0;
}

impl<'m, 'c, T> GearboxSimTrait<'m, 'c> for Simulator<'m, 'c, T>
where
    T: Mapping,
{
    type Mapping = T;

    type SingleResult = SingleResult;

    fn new(
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: &'m sprs::CsMatI<sprs::num_kinds::Pattern, u32>,
        config: &'c crate::pim::configv2::ConfigV2,
        mapping: Self::Mapping,
    ) -> Self {
        Self {
            mapping,
            result: Default::default(),
            matrix_b,
            config,
            evil_col_ids: evil_col_ids.into_iter().collect(),
            evil_row_ids: evil_row_ids.into_iter().collect(),
        }
    }
    fn handle_distribute_default(
        &mut self,
        target_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        mat_b_col_id: LogicColId,
        distribute_type: DistributeType,
    ) {
        match distribute_type {
            DistributeType::EvilRow => {
                todo!()
            }
            DistributeType::EvilCol => todo!(),
            DistributeType::Local => todo!(),
            DistributeType::Remote => todo!(),
        };
    }
    fn report(&self, _name: String, _batch: usize, _topk: f32) -> Self::SingleResult {
        self.result.clone()
    }

    fn get_evil_row_ids(&self) -> usize {
        self.evil_col_ids.len()
    }

    fn get_evil_col_ids(&self) -> usize {
        self.evil_row_ids.len()
    }

    fn get_mapping(&self) -> &Self::Mapping {
        &self.mapping
    }

    fn evil_row_contains(&self, row_id: usize) -> bool {
        self.evil_row_ids.contains(&row_id)
    }

    fn evil_col_contains(&self, col_id: usize) -> bool {
        self.evil_col_ids.contains(&col_id)
    }

    fn get_matrix_b(&self) -> &CsMatI<Pattern, u32> {
        &self.matrix_b
    }
}
