use windows::Win32::{
    Foundation::HANDLE,
    System::{
        Diagnostics::Debug::{FlushInstructionCache, ReadProcessMemory, WriteProcessMemory},
        Memory::{VirtualProtectEx, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS},
    },
};

pub struct Breakpoint {
    process: HANDLE,
    address: u64,
    type_: BreakpointType,
    original_instruction: [u8; 2],
}

impl Breakpoint {
    pub fn new(
        process: HANDLE,
        address: u64,
        type_: BreakpointType,
    ) -> windows::core::Result<Self> {
        let mut original_instruction = [0u8; 2];
        unsafe {
            let _ = ReadProcessMemory(
                process,
                address as *const _,
                original_instruction.as_mut_ptr() as *mut _,
                type_.instruction().len(),
                None,
            )
                .ok()?;
        }
        Ok(Self {
            process,
            address,
            type_,
            original_instruction,
        })
    }

    pub fn enable(&self) {
        unsafe {
            let instruction_length = self.type_.instruction().len();
            let mut original_protection_flags = PAGE_PROTECTION_FLAGS::default();
            if VirtualProtectEx(
                self.process,
                self.address as *const _,
                instruction_length,
                PAGE_EXECUTE_READWRITE,
                &mut original_protection_flags,
            )
                .ok()
                .is_err()
            {
                return;
            }
            let _: windows::core::Result<()> = try {
                WriteProcessMemory(
                    self.process,
                    self.address as *const _,
                    self.type_.instruction().as_ptr() as *const _,
                    instruction_length,
                    None,
                )
                    .ok()?;
                FlushInstructionCache(
                    self.process,
                    Some(self.address as *const _),
                    instruction_length,
                )
                    .ok()?;
            };
            let _ = VirtualProtectEx(
                self.process,
                self.address as *const _,
                instruction_length,
                original_protection_flags,
                &mut original_protection_flags,
            )
                .ok();
        }
    }

    pub fn disable(&self) {
        unsafe {
            let instruction_length = self.type_.instruction().len();
            let mut original_protection_flags = PAGE_PROTECTION_FLAGS::default();
            if VirtualProtectEx(
                self.process,
                self.address as *const _,
                instruction_length,
                PAGE_EXECUTE_READWRITE,
                &mut original_protection_flags,
            )
                .ok()
                .is_err()
            {
                return;
            }
            let _: windows::core::Result<()> = try {
                WriteProcessMemory(
                    self.process,
                    self.address as *const _,
                    self.original_instruction.as_ptr() as *const _,
                    instruction_length,
                    None,
                )
                    .ok()?;
                FlushInstructionCache(
                    self.process,
                    Some(self.address as *const _),
                    instruction_length,
                )
                    .ok()?;
            };
            let _ = VirtualProtectEx(
                self.process,
                self.address as *const _,
                instruction_length,
                original_protection_flags,
                &mut original_protection_flags,
            )
                .ok();
        }
    }
}

impl Drop for Breakpoint {
    fn drop(&mut self) {
        self.disable();
    }
}

#[derive(Clone)]
pub enum BreakpointType {
    Int3,
    Int3Imm,
    Ud2,
}

impl BreakpointType {
    fn instruction(&self) -> &[u8] {
        match self {
            BreakpointType::Int3 => &[0xCC],
            BreakpointType::Int3Imm => &[0xCD, 0x03],
            BreakpointType::Ud2 => &[0x0F, 0x0B],
        }
    }
}
