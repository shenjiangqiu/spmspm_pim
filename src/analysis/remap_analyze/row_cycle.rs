use super::jump::*;
use spmspm_macro::jump_cycles;

jump_cycles!(
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let types = JumpCyclesTypes::NormalJumpCycle256;

        for i in types {
            println!("{:?}", i);
        }
    }
}
