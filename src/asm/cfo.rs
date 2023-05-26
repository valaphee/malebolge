use iced_x86::{Decoder, FlowControl, Instruction, OpKind};

pub struct Cfo<'a, 'b> {
    decoder: &'b mut Decoder<'a>,
}

impl<'a, 'b> Cfo<'a, 'b> {
    pub fn new(decoder: &'b mut Decoder<'a>) -> Self {
        Self { decoder }
    }
}

impl Iterator for Cfo<'_, '_> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.decoder.can_decode() {
            let instruction = self.decoder.decode();
            match instruction.flow_control() {
                FlowControl::ConditionalBranch | FlowControl::UnconditionalBranch => {
                    if instruction.op0_kind() == OpKind::NearBranch64 && instruction.len() == 2 {
                        let align_address = instruction.near_branch_target();
                        let mut align = false;
                        if self.decoder.ip() < align_address {
                            align = true;

                            // store address and position
                            let address = self.decoder.ip();
                            let position = self.decoder.position();

                            // check if branch target is aligned, or a contrary instruction is found
                            let mut instruction_peek = Instruction::default();
                            while self.decoder.can_decode()
                                && self.decoder.ip() < align_address
                                && align
                            {
                                self.decoder.decode_out(&mut instruction_peek);
                                if self.decoder.ip() == align_address
                                    || instruction.code()
                                        == instruction_peek.code().negate_condition_code()
                                {
                                    align = false;
                                }
                            }

                            // restore address and position
                            self.decoder.set_ip(address);
                            self.decoder.set_position(position).unwrap();
                        }
                        if align {
                            self.decoder.set_ip(align_address);
                            self.decoder
                                .set_position(
                                    self.decoder.position()
                                        + (align_address - instruction.ip()) as usize
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
