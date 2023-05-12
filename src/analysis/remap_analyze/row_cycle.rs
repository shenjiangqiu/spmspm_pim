//! ## rust module
//! ## Author: Jiangqiu Shen
//! ## Date: 2023-05-11
//! Description: define all the row cycles that are going to be evaluated
//!
use super::jump::*;
use spmspm_macro::jump_cycles;
jump_cycles!(
    AllJumpCycles;
    NormalJumpCycle<256>,
    NormalJumpCycle<128>,
    NormalJumpCycle<64>,
    NormalJumpCycle<32>,
    IdealJumpCycle<256>,
    IdealJumpCycle<128>,
    IdealJumpCycle<64>,
    IdealJumpCycle<32>,
    MyJumpCycle<16, 256>,
    MyJumpCycle<16, 128>,
    MyJumpCycle<16, 64>,
    MyJumpCycle<16, 32>,
    MyJumpNoOverhead<16, 256>,
    MyJumpNoOverhead<16, 128>,
    MyJumpNoOverhead<16, 64>,
    MyJumpNoOverhead<16, 32>,
    MyJumpOpt<16, 256>,
    MyJumpOpt<16, 128>,
    MyJumpOpt<16, 64>,
    MyJumpOpt<16, 32>,
    MyJumpOnly<16, 256>,
    MyJumpOnly<16, 128>,
    MyJumpOnly<16, 64>,
    MyJumpOnly<16, 32>,
);

// #[derive(JumpCyclesStruct, Default, Clone, Serialize, Deserialize, Debug)]
// pub struct AllJumpCycles {
//     a1: NormalJumpCycle<256>,
//     a2: NormalJumpCycle<128>,
//     a3: NormalJumpCycle<64>,
//     a4: NormalJumpCycle<32>,
//     a5: IdealJumpCycle<256>,
//     a6: IdealJumpCycle<128>,
//     a7: IdealJumpCycle<64>,
//     a8: IdealJumpCycle<32>,
//     a9: MyJumpCycle<16, 256>,
//     a10: MyJumpCycle<16, 128>,
//     a11: MyJumpCycle<16, 64>,
//     a12: MyJumpCycle<16, 32>,
//     a13: MyJumpNoOverhead<16, 256>,
//     a14: MyJumpNoOverhead<16, 128>,
//     a15: MyJumpNoOverhead<16, 64>,
//     a16: MyJumpNoOverhead<16, 32>,
//     a17: MyJumpOpt<16, 256>,
//     a18: MyJumpOpt<16, 128>,
//     a19: MyJumpOpt<16, 64>,
//     a20: MyJumpOpt<16, 32>,
//     a21: MyJumpOnly<16, 256>,
//     a22: MyJumpOnly<16, 128>,
//     a23: MyJumpOnly<16, 64>,
//     a24: MyJumpOnly<16, 32>,
// }
#[cfg(test)]
mod tests {
    use spmspm_macro::JumpCyclesStruct;

    use super::*;
    #[test]
    fn test() {
        let types = AllJumpCyclesTypes::default();

        for i in types {
            println!("{:?}", i);
        }
    }

    #[test]
    #[allow(dead_code, unused_variables)]
    fn test_struct_derive() {
        #[derive(JumpCyclesStruct)]
        pub struct TestStruct {
            aa: NormalJumpCycle<32>,
            aab: NormalJumpCycle<32>,
            aab_33_ddaf: NormalJumpCycle<32>,
        }
        impl JumpCycle for NormalJumpCycle<32> {
            fn total(&self) -> usize {
                todo!()
            }

            fn get_one_jump(&self) -> usize {
                todo!()
            }

            fn get_multi_jump(&self) -> usize {
                todo!()
            }

            fn get_one_jump_mut(&mut self) -> &mut usize {
                todo!()
            }

            fn get_multi_jump_mut(&mut self) -> &mut usize {
                todo!()
            }
        }
        impl UpdatableJumpCycle for NormalJumpCycle<32> {
            fn update(
                &mut self,
                row_status: &RowIdWordId,
                loc: &RowLocation,
                size: WordId,
                remap_cycle: usize,
            ) {
                todo!()
            }
        }
        impl AddableJumpCycle for NormalJumpCycle<32> {
            fn add(&mut self, jump_cycle: &Self) {
                todo!()
            }
        }
        let _ = TestStruct {
            aa: NormalJumpCycle::<32>::default(),
            aab: NormalJumpCycle::<32>::default(),
            aab_33_ddaf: NormalJumpCycle::<32>::default(),
        };
        let a_types = TestStructTypes::default();
        for t in a_types {
            println!("{:?}", t);
        }
    }
}
