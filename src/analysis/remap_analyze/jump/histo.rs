use serde::{Deserialize, Serialize};

use crate::analysis::{
    mapping::{PhysicRowId, WordId},
    translate_mapping::RowLocation,
};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct HistoJump {
    j_0: usize,
    j_1: usize,
    j_2: usize,
    j_4: usize,
    j_8: usize,
    j_16: usize,
    j_32: usize,

    j_0_df_row: usize,
    j_1_df_row: usize,
    j_2_df_row: usize,
    j_4_df_row: usize,
    j_8_df_row: usize,
    j_16_df_row: usize,
    j_32_df_row: usize,
}
impl super::JumpCycle for HistoJump {
    fn total(&self) -> usize {
        0
    }

    fn get_one_jump(&self) -> usize {
        0
    }

    fn get_multi_jump(&self) -> usize {
        0
    }

    fn get_one_jump_mut(&mut self) -> &mut usize {
        &mut self.j_0
    }

    fn get_multi_jump_mut(&mut self) -> &mut usize {
        &mut self.j_0
    }
}

impl super::AddableJumpCycle for HistoJump {
    fn add(&mut self, jump_cycle: &Self) {
        self.j_0 += jump_cycle.j_0;
        self.j_1 += jump_cycle.j_1;
        self.j_2 += jump_cycle.j_2;
        self.j_4 += jump_cycle.j_4;
        self.j_8 += jump_cycle.j_8;
        self.j_16 += jump_cycle.j_16;
        self.j_32 += jump_cycle.j_32;

        self.j_0_df_row += jump_cycle.j_0_df_row;
        self.j_1_df_row += jump_cycle.j_1_df_row;
        self.j_2_df_row += jump_cycle.j_2_df_row;
        self.j_4_df_row += jump_cycle.j_4_df_row;
        self.j_8_df_row += jump_cycle.j_8_df_row;
        self.j_16_df_row += jump_cycle.j_16_df_row;
        self.j_32_df_row += jump_cycle.j_32_df_row;
    }
}

impl super::UpdatableJumpCycle for HistoJump {
    fn update(
        &mut self,
        row_status: &(PhysicRowId, WordId),
        loc: &RowLocation,
        _size: WordId,
        _remap_cycle: usize,
    ) {
        let gap = (loc.word_id.0 as isize - row_status.1 .0 as isize).abs() as usize;
        if loc.row_id == row_status.0 {
            match gap {
                0 => {
                    self.j_0 += gap;
                }
                1 => {
                    self.j_1 += gap;
                }
                2 => {
                    self.j_2 += gap;
                }
                3..=4 => {
                    self.j_4 += gap;
                }
                5..=8 => {
                    self.j_8 += gap;
                }
                9..=16 => {
                    self.j_16 += gap;
                }
                _ => {
                    self.j_32 += gap;
                }
            }
        } else {
            match gap {
                0 => {
                    self.j_0_df_row += gap;
                }
                1 => {
                    self.j_1_df_row += gap;
                }
                2 => {
                    self.j_2_df_row += gap;
                }
                3..=4 => {
                    self.j_4_df_row += gap;
                }
                5..=8 => {
                    self.j_8_df_row += gap;
                }
                9..=16 => {
                    self.j_16_df_row += gap;
                }
                _ => {
                    self.j_32_df_row += gap;
                }
            }
        }
    }
}
