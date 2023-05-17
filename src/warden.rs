use iced_x86::{Decoder, FlowControl, Instruction, OpKind};

pub struct CfoPatcher<'a, 'b> {
    decoder: &'b mut Decoder<'a>,
}

impl<'a, 'b> CfoPatcher<'a, 'b> {
    pub fn new(decoder: &'b mut Decoder<'a>) -> Self {
        Self { decoder }
    }
}

impl Iterator for CfoPatcher<'_, '_> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.decoder.can_decode() {
            let mut instruction = self.decoder.decode();
            match instruction.flow_control() {
                FlowControl::ConditionalBranch | FlowControl::UnconditionalBranch => {
                    if instruction.op0_kind() == OpKind::NearBranch64 && instruction.len() == 2 {
                        let branch_target = instruction.near_branch_target();
                        // check if branch target is negative
                        let align = if self.decoder.ip() >= branch_target {
                            false
                        } else {
                            let mut align = true;
                            // save address and position
                            let address = self.decoder.ip();
                            let position = self.decoder.position();
                            // check if branch target is aligned, or a contrary instruction is found
                            let mut instruction_peek = Instruction::default();
                            while self.decoder.can_decode() && self.decoder.ip() < branch_target && align {
                                self.decoder.decode_out(&mut instruction_peek);
                                if self.decoder.ip() == branch_target || instruction.code() == instruction_peek.code().negate_condition_code() {
                                    align = false;
                                }
                            }
                            // restore address and position
                            self.decoder.set_ip(address);
                            self.decoder.set_position(position).unwrap();
                            align
                        };
                        if align {
                            self.decoder.set_ip(branch_target);
                            self.decoder
                                .set_position(
                                    self.decoder.position()
                                        + (branch_target - instruction.ip()) as usize
                                        - instruction.len(),
                                )
                                .unwrap();
                            return self.next();
                        }
                    }
                }
                _ => {}
            }
            Some(instruction)
        } else {
            None
        }
    }
}
