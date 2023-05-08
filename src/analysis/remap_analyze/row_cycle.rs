use serde::{Deserialize, Serialize};

use crate::analysis::{
    mapping::{PhysicRowId, WordId},
    translate_mapping::RowLocation,
};

use super::jump::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JumpTypes {
    Normal = 0,
    Ideal,
    // FromSource,
    My16,
    My32,
    My64,
    My16NoOverhead,
    My32NoOverhead,
    My64NoOverhead,
    My16Opt,
    My32Opt,
    My64Opt,
    // Smart,
    // Histo,
    End,
}
pub struct JumpTypeIterator {
    jump_type: JumpTypes,
}
impl JumpTypeIterator {
    pub fn new() -> Self {
        Self {
            jump_type: JumpTypes::Normal,
        }
    }
}
impl Iterator for JumpTypeIterator {
    type Item = JumpTypes;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.jump_type;
        self.jump_type.next();
        if current == JumpTypes::End {
            None
        } else {
            Some(current)
        }
    }
}

// impl From<usize> for JumpTypes {
//     fn from(value: usize) -> Self {
//         match value {
//             0 => JumpTypes::Normal,
//             1 => JumpTypes::Ideal,
//             2 => JumpTypes::FromSource,
//             3 => JumpTypes::My16,
//             4 => JumpTypes::My32,
//             5 => JumpTypes::My64,
//             6 => JumpTypes::My16NoOverhead,
//             7 => JumpTypes::My32NoOverhead,
//             8 => JumpTypes::My64NoOverhead,
//             9 => JumpTypes::My16Opt,
//             10 => JumpTypes::My32Opt,
//             11 => JumpTypes::My64Opt,
//             12 => JumpTypes::Smart,
//             13 => JumpTypes::End,
//             // 14 => JumpTypes::Histo,
//             _ => panic!("Invalid JumpTypes"),
//         }
//     }
// }
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct RowCycle {
    pub normal_jump_cycle: NormalJumpCycle,
    pub ideal_jump_cycle: IdealJumpCycle,
    // pub from_source_jump_cycle: FromSourceJumpCycle,
    pub my_jump_cycle_16: MyJumpCycle<16>,
    pub my_jump_cycle_32: MyJumpCycle<32>,
    pub my_jump_cycle_64: MyJumpCycle<64>,
    pub my_jump_cycle_16_no_overhead: MyJumpNoOverhead<16>,
    pub my_jump_cycle_64_no_overhead: MyJumpNoOverhead<64>,
    pub my_jump_cycle_32_no_overhead: MyJumpNoOverhead<32>,
    pub my_jump_cycle_16_opt: MyJumpOpt<16>,
    pub my_jump_cycle_32_opt: MyJumpOpt<32>,
    pub my_jump_cycle_64_opt: MyJumpOpt<64>,
    // pub histo: HistoJump,
    // pub smart_jump_cycle: SmartJumpCycle,
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
            // JumpTypes::FromSource => &mut self.row_cycle.from_source_jump_cycle,
            JumpTypes::My16 => &mut self.row_cycle.my_jump_cycle_16,
            JumpTypes::My32 => &mut self.row_cycle.my_jump_cycle_32,
            JumpTypes::My64 => &mut self.row_cycle.my_jump_cycle_64,
            JumpTypes::My16NoOverhead => &mut self.row_cycle.my_jump_cycle_16_no_overhead,
            JumpTypes::My32NoOverhead => &mut self.row_cycle.my_jump_cycle_32_no_overhead,
            JumpTypes::My64NoOverhead => &mut self.row_cycle.my_jump_cycle_64_no_overhead,
            JumpTypes::My16Opt => &mut self.row_cycle.my_jump_cycle_16_opt,
            JumpTypes::My32Opt => &mut self.row_cycle.my_jump_cycle_32_opt,
            JumpTypes::My64Opt => &mut self.row_cycle.my_jump_cycle_64_opt,
            // JumpTypes::Smart => &mut self.row_cycle.smart_jump_cycle,
            // JumpTypes::Histo => &mut self.row_cycle.histo,
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
            // JumpTypes::FromSource => self.row_cycle.from_source_jump_cycle.total(),
            JumpTypes::My16 => self.row_cycle.my_jump_cycle_16.total(),
            JumpTypes::My32 => self.row_cycle.my_jump_cycle_32.total(),
            JumpTypes::My64 => self.row_cycle.my_jump_cycle_64.total(),
            JumpTypes::My16NoOverhead => self.row_cycle.my_jump_cycle_16_no_overhead.total(),
            JumpTypes::My32NoOverhead => self.row_cycle.my_jump_cycle_32_no_overhead.total(),
            JumpTypes::My64NoOverhead => self.row_cycle.my_jump_cycle_64_no_overhead.total(),
            JumpTypes::My16Opt => self.row_cycle.my_jump_cycle_16_opt.total(),
            JumpTypes::My32Opt => self.row_cycle.my_jump_cycle_32_opt.total(),
            JumpTypes::My64Opt => self.row_cycle.my_jump_cycle_64_opt.total(),
            // JumpTypes::Smart => self.row_cycle.smart_jump_cycle.total(),
            // JumpTypes::Histo => self.row_cycle.histo.total(),
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
            // JumpTypes::FromSource => (
            //     self.row_cycle.from_source_jump_cycle.get_one_jump(),
            //     self.row_cycle.from_source_jump_cycle.get_multi_jump(),
            // ),
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
            // JumpTypes::Smart => (
            //     self.row_cycle.smart_jump_cycle.get_one_jump(),
            //     self.row_cycle.smart_jump_cycle.get_multi_jump(),
            // ),
            // JumpTypes::Histo => (
            //     self.row_cycle.histo.get_one_jump(),
            //     self.row_cycle.histo.get_multi_jump(),
            // ),
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
            JumpTypes::Ideal => JumpTypes::My16,
            // JumpTypes::Ideal => JumpTypes::FromSource,
            // JumpTypes::FromSource => JumpTypes::My16,
            JumpTypes::My16 => JumpTypes::My32,
            JumpTypes::My32 => JumpTypes::My64,
            JumpTypes::My64 => JumpTypes::My16NoOverhead,
            JumpTypes::My16NoOverhead => JumpTypes::My32NoOverhead,
            JumpTypes::My32NoOverhead => JumpTypes::My64NoOverhead,
            JumpTypes::My64NoOverhead => JumpTypes::My16Opt,
            JumpTypes::My16Opt => JumpTypes::My32Opt,
            JumpTypes::My32Opt => JumpTypes::My64Opt,
            JumpTypes::My64Opt => JumpTypes::End,
            // JumpTypes::Smart => JumpTypes::End,
            // JumpTypes::Histo =>
            JumpTypes::End => JumpTypes::End,
        }
    }
}
struct AddTwoIterator<'a> {
    target: &'a mut RowCycle,
    source: &'a RowCycle,
    index: JumpTypes,
}
impl<'iter> GatIterator for AddTwoIterator<'iter> {
    type Item<'a> = Box<dyn AddTwo+'a> where Self: 'a;

    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>> {
        let cycle: Self::Item<'a> = match self.index {
            JumpTypes::Normal => Box::new(TargetSourcePair {
                target: &mut self.target.normal_jump_cycle,
                source: &self.source.normal_jump_cycle,
            }),
            JumpTypes::Ideal => Box::new(TargetSourcePair {
                target: &mut self.target.ideal_jump_cycle,
                source: &self.source.ideal_jump_cycle,
            }),
            // JumpTypes::FromSource => Box::new(TargetSourcePair {
            //     target: &mut self.target.from_source_jump_cycle,
            //     source: &self.source.from_source_jump_cycle,
            // }),
            JumpTypes::My16 => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_16,
                source: &self.source.my_jump_cycle_16,
            }),
            JumpTypes::My32 => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_32,
                source: &self.source.my_jump_cycle_32,
            }),
            JumpTypes::My64 => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_64,
                source: &self.source.my_jump_cycle_64,
            }),
            JumpTypes::My16NoOverhead => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_16_no_overhead,
                source: &self.source.my_jump_cycle_16_no_overhead,
            }),
            JumpTypes::My32NoOverhead => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_32_no_overhead,
                source: &self.source.my_jump_cycle_32_no_overhead,
            }),
            JumpTypes::My64NoOverhead => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_64_no_overhead,
                source: &self.source.my_jump_cycle_64_no_overhead,
            }),
            JumpTypes::My16Opt => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_16_opt,
                source: &self.source.my_jump_cycle_16_opt,
            }),
            JumpTypes::My32Opt => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_32_opt,
                source: &self.source.my_jump_cycle_32_opt,
            }),
            JumpTypes::My64Opt => Box::new(TargetSourcePair {
                target: &mut self.target.my_jump_cycle_64_opt,
                source: &self.source.my_jump_cycle_64_opt,
            }),
            // JumpTypes::Smart => Box::new(TargetSourcePair {
            //     target: &mut self.target.smart_jump_cycle,
            //     source: &self.source.smart_jump_cycle,
            // }),
            // JumpTypes::Histo => Box::new(TargetSourcePair {
            //     target: &mut self.target.histo,
            //     source: &self.source.histo,
            // }),
            JumpTypes::End => return None,
        };
        self.index.next();
        Some(cycle)
    }
}

impl RowCycle {
    pub(crate) fn update(
        &mut self,
        row_status: &(PhysicRowId, WordId),
        location: &RowLocation,
        word_size: WordId,
        remap_cycle: usize,
    ) {
        let mut update_iter = self.update_iter_mut();
        while let Some(jump_cycle) = update_iter.next() {
            jump_cycle.update(row_status, location, word_size, remap_cycle);
        }
    }
}
