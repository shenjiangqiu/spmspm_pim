use std::{borrow::Borrow, collections::BTreeMap};

use itertools::Itertools;

use crate::analysis::remap_analyze::row_cycle::*;

pub struct SelectiveUpdator<'a, const WALKER_SIZE: usize, T: UpdatableJumpCycle> {
    pub extra_scan_cycles: usize,
    pub jump_cycle: &'a mut T,
    pub row_status: RowIdWordId,
    pub size: WordId,
    pub remap_cycle: usize,
}
impl<'a, const WALKER_SIZE: usize, T: UpdatableJumpCycle> SelectiveUpdator<'a, WALKER_SIZE, T> {
    pub fn new(
        jump_cycle: &'a mut T,
        row_status: RowIdWordId,
        size: WordId,
        remap_cycle: usize,
    ) -> Self {
        Self {
            extra_scan_cycles: 0,
            jump_cycle,
            row_status,
            size,
            remap_cycle,
        }
    }
}

impl<const WALKER_SIZE: usize, T: UpdatableJumpCycle> super::RemoteUpdator
    for SelectiveUpdator<'_, WALKER_SIZE, T>
{
    fn update<Item: Borrow<RowLocation>>(&mut self, data: impl IntoIterator<Item = Item>) {
        // first we need to split the tasks into different walkers, each data is (col,data) is 8 Bytes, so there are WALKER_SIZE/8 tasks in one walker
        for task in data.into_iter().chunks(WALKER_SIZE / 8).into_iter() {
            // group the tasks by row_id and walker_id
            // we need btree map here because the key sequence is important!! (from small to large)
            let tasks: BTreeMap<_, _> = task
                .into_group_map_by(|x| {
                    (
                        x.borrow().row_id_word_id.row_id,
                        x.borrow().row_id_word_id.word_id.0 * 4 / WALKER_SIZE,
                    )
                })
                .into_iter()
                .collect();

            if tasks.is_empty() {
                continue;
            }
            self.extra_scan_cycles += (tasks.len() - 1) * WALKER_SIZE / 8;

            // for each round, update the result
            for ((_row_id, _walker_id), locs) in tasks {
                for loc in locs {
                    self.jump_cycle.update(
                        &self.row_status,
                        loc.borrow(),
                        self.size,
                        self.remap_cycle,
                    );
                    self.row_status = loc.borrow().row_id_word_id;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use crate::analysis::remap_analyze::{jump::NormalJumpCycle, remote_updator::RemoteUpdator};

    #[test]
    fn test_borrow() {
        struct A {
            a: usize,
            b: u32,
        }
        impl Borrow<usize> for A {
            fn borrow(&self) -> &usize {
                &self.a
            }
        }
        impl Borrow<u32> for A {
            fn borrow(&self) -> &u32 {
                &self.b
            }
        }
        let a = A { a: 1, b: 2 };
        let b: &usize = a.borrow();
        let c: &u32 = a.borrow();
        assert_eq!(*b, 1);
        assert_eq!(*c, 2);
    }

    #[test]
    fn test_selective() {
        use super::*;
        let mut normal_jump_cycle = NormalJumpCycle::<32>::default();
        let mut selective_updator = SelectiveUpdator::<'_, 32, _>::new(
            &mut normal_jump_cycle,
            RowIdWordId {
                row_id: 0.into(),
                word_id: 0.into(),
            },
            WordId(1),
            1,
        );
        let row_locations = [1, 9, 1, 9, 1, 9, 1, 9].into_iter().map(|x| RowLocation {
            row_id_word_id: RowIdWordId {
                row_id: 0.into(),
                word_id: WordId(x),
            },
            subarray_id: 0.into(),
        });
        selective_updator.update(row_locations);
        println!("extra scan cycles: {}", selective_updator.extra_scan_cycles);
        println!("jump cycles: {:#?}", normal_jump_cycle);
    }
    #[test]
    fn test_sequential() {
        use super::*;
        let mut normal_jump_cycle = NormalJumpCycle::<32>::default();
        let mut row_status = RowIdWordId {
            row_id: 0.into(),
            word_id: 0.into(),
        };
        let row_locations = [1, 9, 1, 9, 1, 9, 1, 9].into_iter().map(|x| RowLocation {
            row_id_word_id: RowIdWordId {
                row_id: 0.into(),
                word_id: WordId(x),
            },
            subarray_id: 0.into(),
        });
        for row_location in row_locations {
            normal_jump_cycle.update(&row_status, &row_location, 1.into(), 1);
            row_status = row_location.row_id_word_id;
        }
        println!("jump cycles: {:#?}", normal_jump_cycle);
    }

    #[test]
    #[ignore]
    fn test_offshore() {
        // there is the bug in the offshore, test it here!!!!
        // use super::*;
        // let mut normal_jump_cycle = NormalJumpCycle::<32>::default();
        // let mut normal_jump_selective = NormalJumpCycleSelective::<32>::default();
        // let graph: CsMatI<Pattern, u32> =
        //     sprs::io::read_matrix_market("mtx/outerspace/offshore/offshore.mtx")
        //         .unwrap()
        //         .to_csr();
        // let config = ConfigV3::new("configs/real_jump_same_bank-1-16.toml");
        // let mapping = SameBankMapping::new(
        //     config.banks.num,
        //     config.channels.num,
        //     config.subarrays,
        //     config.gearbox_config.,
        //     cols,
        //     graph,
        //     graph_csr,
        // );
        // let result = run_with_mapping(mapping, config, matrix_csr, algorithm, max_rounds);
    }
}
