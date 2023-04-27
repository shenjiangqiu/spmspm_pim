use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::jump::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JumpTypes {
    Normal = 0,
    Ideal,
    FromSource,
    My16,
    My32,
    My64,
    My16NoOverhead,
    My32NoOverhead,
    My64NoOverhead,
    My16Opt,
    My32Opt,
    My64Opt,
    Smart,
    End,
}

impl From<usize> for JumpTypes {
    fn from(value: usize) -> Self {
        match value {
            0 => JumpTypes::Normal,
            1 => JumpTypes::Ideal,
            2 => JumpTypes::FromSource,
            3 => JumpTypes::My16,
            4 => JumpTypes::My32,
            5 => JumpTypes::My64,
            6 => JumpTypes::My16NoOverhead,
            7 => JumpTypes::My32NoOverhead,
            8 => JumpTypes::My64NoOverhead,
            9 => JumpTypes::My16Opt,
            10 => JumpTypes::My32Opt,
            11 => JumpTypes::My64Opt,
            12 => JumpTypes::Smart,
            13 => JumpTypes::End,
            _ => panic!("Invalid JumpTypes"),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct RowCycle {
    pub normal_jump_cycle: NormalJumpCycle,
    pub ideal_jump_cycle: IdealJumpCycle,
    pub from_source_jump_cycle: FromSourceJumpCycle,
    pub my_jump_cycle_16: MyJumpCycle<16>,
    pub my_jump_cycle_32: MyJumpCycle<32>,
    pub my_jump_cycle_64: MyJumpCycle<64>,
    pub my_jump_cycle_16_no_overhead: MyJumpNoOverhead<16>,
    pub my_jump_cycle_64_no_overhead: MyJumpNoOverhead<64>,
    pub my_jump_cycle_32_no_overhead: MyJumpNoOverhead<32>,
    pub my_jump_cycle_16_opt: MyJumpOpt<16>,
    pub my_jump_cycle_32_opt: MyJumpOpt<32>,
    pub my_jump_cycle_64_opt: MyJumpOpt<64>,
    pub smart_jump_cycle: SmartJumpCycle,
}

struct UpdatableRowCycleIterator<'a> {
    row_cycle: &'a mut RowCycle,
    jump_type: JumpTypes,
}
pub(crate) trait GatIterator {
    type Item<'a>
    where
        Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

impl<'row> GatIterator for UpdatableRowCycleIterator<'row> {
    type Item<'a> = &'a mut dyn UpdatableJumpCycle where Self: 'a;
    fn next<'a>(self: &'a mut Self) -> Option<Self::Item<'a>> {
        let cycle: &'a mut dyn UpdatableJumpCycle = match self.jump_type {
            JumpTypes::Normal => &mut self.row_cycle.normal_jump_cycle,
            JumpTypes::Ideal => &mut self.row_cycle.ideal_jump_cycle,
            JumpTypes::FromSource => &mut self.row_cycle.from_source_jump_cycle,
            JumpTypes::My16 => &mut self.row_cycle.my_jump_cycle_16,
            JumpTypes::My32 => &mut self.row_cycle.my_jump_cycle_32,
            JumpTypes::My64 => &mut self.row_cycle.my_jump_cycle_64,
            JumpTypes::My16NoOverhead => &mut self.row_cycle.my_jump_cycle_16_no_overhead,
            JumpTypes::My32NoOverhead => &mut self.row_cycle.my_jump_cycle_32_no_overhead,
            JumpTypes::My64NoOverhead => &mut self.row_cycle.my_jump_cycle_64_no_overhead,
            JumpTypes::My16Opt => &mut self.row_cycle.my_jump_cycle_16_opt,
            JumpTypes::My32Opt => &mut self.row_cycle.my_jump_cycle_32_opt,
            JumpTypes::My64Opt => &mut self.row_cycle.my_jump_cycle_64_opt,
            JumpTypes::Smart => &mut self.row_cycle.smart_jump_cycle,
            JumpTypes::End => return None,
        };
        self.jump_type.next();
        Some(cycle)
    }
}

pub struct RowCycleIterator {
    row_cycle: RowCycle,
    jump_type: JumpTypes,
}
impl IntoIterator for RowCycle {
    type Item = usize;
    type IntoIter = RowCycleIterator;
    fn into_iter(self) -> Self::IntoIter {
        RowCycleIterator {
            row_cycle: self,
            jump_type: JumpTypes::Normal,
        }
    }
}
impl Iterator for RowCycleIterator {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let ret = match self.jump_type {
            JumpTypes::Normal => self.row_cycle.normal_jump_cycle.total(),
            JumpTypes::Ideal => self.row_cycle.ideal_jump_cycle.total(),
            JumpTypes::FromSource => self.row_cycle.from_source_jump_cycle.total(),
            JumpTypes::My16 => self.row_cycle.my_jump_cycle_16.total(),
            JumpTypes::My32 => self.row_cycle.my_jump_cycle_32.total(),
            JumpTypes::My64 => self.row_cycle.my_jump_cycle_64.total(),
            JumpTypes::My16NoOverhead => self.row_cycle.my_jump_cycle_16_no_overhead.total(),
            JumpTypes::My32NoOverhead => self.row_cycle.my_jump_cycle_32_no_overhead.total(),
            JumpTypes::My64NoOverhead => self.row_cycle.my_jump_cycle_64_no_overhead.total(),
            JumpTypes::My16Opt => self.row_cycle.my_jump_cycle_16_opt.total(),
            JumpTypes::My32Opt => self.row_cycle.my_jump_cycle_32_opt.total(),
            JumpTypes::My64Opt => self.row_cycle.my_jump_cycle_64_opt.total(),
            JumpTypes::Smart => self.row_cycle.smart_jump_cycle.total(),
            JumpTypes::End => return None,
        };
        self.jump_type.next();
        Some(ret)
    }
}
impl RowCycle {
    pub fn into_split_iter(self) -> RowCycleSplitIter {
        RowCycleSplitIter {
            row_cycle: self,
            jump_type: JumpTypes::Normal,
        }
    }
    fn update_iter_mut(&mut self) -> UpdatableRowCycleIterator {
        UpdatableRowCycleIterator {
            row_cycle: self,
            jump_type: JumpTypes::Normal,
        }
    }
}
pub struct RowCycleSplitIter {
    row_cycle: RowCycle,
    jump_type: JumpTypes,
}
impl Iterator for RowCycleSplitIter {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let cycle = match self.jump_type {
            JumpTypes::Normal => (
                self.row_cycle.normal_jump_cycle.get_one_jump(),
                self.row_cycle.normal_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::Ideal => (
                self.row_cycle.ideal_jump_cycle.get_one_jump(),
                self.row_cycle.ideal_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::FromSource => (
                self.row_cycle.from_source_jump_cycle.get_one_jump(),
                self.row_cycle.from_source_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::My16 => (
                self.row_cycle.my_jump_cycle_16.get_one_jump(),
                self.row_cycle.my_jump_cycle_16.get_multi_jump(),
            ),
            JumpTypes::My32 => (
                self.row_cycle.my_jump_cycle_32.get_one_jump(),
                self.row_cycle.my_jump_cycle_32.get_multi_jump(),
            ),
            JumpTypes::My64 => (
                self.row_cycle.my_jump_cycle_64.get_one_jump(),
                self.row_cycle.my_jump_cycle_64.get_multi_jump(),
            ),
            JumpTypes::My16NoOverhead => (
                self.row_cycle.my_jump_cycle_16_no_overhead.get_one_jump(),
                self.row_cycle.my_jump_cycle_16_no_overhead.get_multi_jump(),
            ),
            JumpTypes::My32NoOverhead => (
                self.row_cycle.my_jump_cycle_32_no_overhead.get_one_jump(),
                self.row_cycle.my_jump_cycle_32_no_overhead.get_multi_jump(),
            ),
            JumpTypes::My64NoOverhead => (
                self.row_cycle.my_jump_cycle_64_no_overhead.get_one_jump(),
                self.row_cycle.my_jump_cycle_64_no_overhead.get_multi_jump(),
            ),
            JumpTypes::My16Opt => (
                self.row_cycle.my_jump_cycle_16_opt.get_one_jump(),
                self.row_cycle.my_jump_cycle_16_opt.get_multi_jump(),
            ),
            JumpTypes::My32Opt => (
                self.row_cycle.my_jump_cycle_32_opt.get_one_jump(),
                self.row_cycle.my_jump_cycle_32_opt.get_multi_jump(),
            ),
            JumpTypes::My64Opt => (
                self.row_cycle.my_jump_cycle_64_opt.get_one_jump(),
                self.row_cycle.my_jump_cycle_64_opt.get_multi_jump(),
            ),
            JumpTypes::Smart => (
                self.row_cycle.smart_jump_cycle.get_one_jump(),
                self.row_cycle.smart_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::End => return None,
        };
        self.jump_type.next();
        Some(cycle)
    }
}

impl JumpTypes {
    fn next(&mut self) {
        *self = match self {
            JumpTypes::Normal => JumpTypes::Ideal,
            JumpTypes::Ideal => JumpTypes::FromSource,
            JumpTypes::FromSource => JumpTypes::My16,
            JumpTypes::My16 => JumpTypes::My32,
            JumpTypes::My32 => JumpTypes::My64,
            JumpTypes::My64 => JumpTypes::My16NoOverhead,
            JumpTypes::My16NoOverhead => JumpTypes::My32NoOverhead,
            JumpTypes::My32NoOverhead => JumpTypes::My64NoOverhead,
            JumpTypes::My64NoOverhead => JumpTypes::My16Opt,
            JumpTypes::My16Opt => JumpTypes::My32Opt,
            JumpTypes::My32Opt => JumpTypes::My64Opt,
            JumpTypes::My64Opt => JumpTypes::Smart,
            JumpTypes::Smart => JumpTypes::End,
            JumpTypes::End => JumpTypes::End,
        }
    }
}

impl RowCycle {
    pub(crate) fn update(
        &mut self,
        row_status: &(usize, usize),
        location: &RowLocation,
        size: usize,
        remap_cycle: usize,
    ) {
        let mut update_iter = self.update_iter_mut();
        while let Some(jump_cycle) = update_iter.next() {
            jump_cycle.update(row_status, location, size, remap_cycle);
        }
    }
}
