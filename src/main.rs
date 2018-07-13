extern crate libwhp;
#[cfg(windows)]
extern crate winapi;

use libwhp::instruction_emulator::*;
use libwhp::*;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write};
use winapi::um::memoryapi::{VirtualAlloc, VirtualFree};
use winapi::um::winnt::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE};

const CPUID_EXT_HYPERVISOR: UINT32 = 1 << 31;

const PDE64_PRESENT: u64 = 1;
const PDE64_RW: u64 = 1 << 1;
const PDE64_USER: u64 = 1 << 2;
const PDE64_PS: u64 = 1 << 7;
const CR4_PAE: u64 = 1 << 5;

const CR0_PE: u64 = 1;
const CR0_MP: u64 = 1 << 1;
const CR0_ET: u64 = 1 << 4;
const CR0_NE: u64 = 1 << 5;
const CR0_WP: u64 = 1 << 16;
const CR0_AM: u64 = 1 << 18;
const CR0_PG: u64 = 1 << 31;
const EFER_LME: u64 = 1 << 8;
const EFER_LMA: u64 = 1 << 10;

fn main() {
    let p: Partition = Partition::new().unwrap();

    let mut property: WHV_PARTITION_PROPERTY = unsafe { std::mem::zeroed() };
    property.ProcessorCount = 1;
    p.set_property(
        WHV_PARTITION_PROPERTY_CODE::WHvPartitionPropertyCodeProcessorCount,
        &property,
    ).unwrap();

    property = unsafe { std::mem::zeroed() };
    // X64MsrExit | X64MsrExit | ExceptionExit
    property.ExtendedVmExits = 7;

    p.set_property(
        WHV_PARTITION_PROPERTY_CODE::WHvPartitionPropertyCodeExtendedVmExits,
        &property,
    ).unwrap();

    let cpuids: [UINT32; 1] = [1];
    p.set_property_cpuid_exits(&cpuids).unwrap();

    let mut cpuid_results: [WHV_X64_CPUID_RESULT; 1] = unsafe { std::mem::zeroed() };

    cpuid_results[0].Function = 0x40000000;
    let mut id_reg_values: [UINT32; 3] = [0; 3];
    let id = "libwhp\0";
    unsafe {
        std::ptr::copy_nonoverlapping(id.as_ptr(), id_reg_values.as_mut_ptr() as *mut u8, id.len());
    }
    cpuid_results[0].Ebx = id_reg_values[0];
    cpuid_results[0].Ecx = id_reg_values[1];
    cpuid_results[0].Edx = id_reg_values[2];

    p.set_property_cpuid_results(&cpuid_results).unwrap();

    p.setup().unwrap();

    let mem_size = 0x200000;
    let mem_addr = unsafe {
        VirtualAlloc(
            std::ptr::null_mut(),
            mem_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };

    let guest_address: WHV_GUEST_PHYSICAL_ADDRESS = 0;

    p.map_gpa_range(
        mem_addr as *const std::os::raw::c_void,
        guest_address,
        mem_size as UINT64,
        WHV_MAP_GPA_RANGE_FLAGS::WHvMapGpaRangeFlagRead
            | WHV_MAP_GPA_RANGE_FLAGS::WHvMapGpaRangeFlagWrite
            | WHV_MAP_GPA_RANGE_FLAGS::WHvMapGpaRangeFlagExecute,
    ).unwrap();

    let vp = p.create_virtual_processor(0).unwrap();

    let pml4_addr: u64 = 0x2000;
    let pdpt_addr: u64 = 0x3000;
    let pd_addr: u64 = 0x4000;
    let pml4: u64 = mem_addr as u64 + pml4_addr;
    let pdpt: u64 = mem_addr as u64 + pdpt_addr;
    let pd: u64 = mem_addr as u64 + pd_addr;

    unsafe {
        *(pml4 as *mut u64) = PDE64_PRESENT | PDE64_RW | PDE64_USER | pdpt_addr;
        *(pdpt as *mut u64) = PDE64_PRESENT | PDE64_RW | PDE64_USER | pd_addr;
        *(pd as *mut u64) = PDE64_PRESENT | PDE64_RW | PDE64_USER | PDE64_PS;
    }

    const NUM_REGS: UINT32 = 10;
    let mut reg_names: [WHV_REGISTER_NAME; NUM_REGS as usize] = unsafe { std::mem::zeroed() };
    let mut reg_values: [WHV_REGISTER_VALUE; NUM_REGS as usize] = unsafe { std::mem::zeroed() };

    reg_names[0] = WHV_REGISTER_NAME::WHvX64RegisterCr3;
    reg_values[0].Reg64 = pml4_addr;
    reg_names[1] = WHV_REGISTER_NAME::WHvX64RegisterCr4;
    reg_values[1].Reg64 = CR4_PAE;
    reg_names[2] = WHV_REGISTER_NAME::WHvX64RegisterCr0;
    reg_values[2].Reg64 = CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG;
    reg_names[3] = WHV_REGISTER_NAME::WHvX64RegisterEfer;
    reg_values[3].Reg64 = EFER_LME | EFER_LMA;

    reg_names[4] = WHV_REGISTER_NAME::WHvX64RegisterCs;
    unsafe {
        reg_values[4].Segment.Base = 0;
        reg_values[4].Segment.Limit = 0xffffffff;
        reg_values[4].Segment.Selector = 1 << 3;
        reg_values[4].Segment.Attributes = 11 + (1 << 7) + (1 << 15) + (1 << 13) + (1 << 4);
    }

    reg_names[5] = WHV_REGISTER_NAME::WHvX64RegisterDs;
    unsafe {
        reg_values[5].Segment.Base = 0;
        reg_values[5].Segment.Limit = 0xffffffff;
        reg_values[5].Segment.Selector = 2 << 3;
        reg_values[5].Segment.Attributes = 3 + (1 << 7) + (1 << 15) + (1 << 13) + (1 << 4);
    }

    reg_names[6] = WHV_REGISTER_NAME::WHvX64RegisterEs;
    reg_values[6] = reg_values[5];

    reg_names[7] = WHV_REGISTER_NAME::WHvX64RegisterFs;
    reg_values[7] = reg_values[5];

    reg_names[8] = WHV_REGISTER_NAME::WHvX64RegisterGs;
    reg_values[8] = reg_values[5];

    reg_names[9] = WHV_REGISTER_NAME::WHvX64RegisterSs;
    reg_values[9] = reg_values[5];

    vp.set_registers(&reg_names, &reg_values).unwrap();

    let mut reg_names: [WHV_REGISTER_NAME; 3 as usize] = unsafe { std::mem::zeroed() };
    let mut reg_values: [WHV_REGISTER_VALUE; 3 as usize] = unsafe { std::mem::zeroed() };

    reg_names[0] = WHV_REGISTER_NAME::WHvX64RegisterRflags;
    reg_values[0].Reg64 = 2;
    reg_names[1] = WHV_REGISTER_NAME::WHvX64RegisterRip;
    reg_values[1].Reg64 = 0;
    reg_names[2] = WHV_REGISTER_NAME::WHvX64RegisterRsp;
    reg_values[2].Reg64 = 2 << 20;

    vp.set_registers(&reg_names, &reg_values).unwrap();

    let mut f = File::open("payload.img").unwrap();
    let slice = unsafe { std::slice::from_raw_parts_mut(mem_addr as *mut u8, mem_size) };
    f.read(slice).unwrap();
    drop(f);

    let mut callbacks = SampleCallbacks { vp: &vp };
    let mut e = Emulator::new(&mut callbacks).unwrap();

    loop {
        let exit_context = vp.run().unwrap();
        // Handle exits
        if exit_context.ExitReason == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonX64Halt {
            break;
        } else if exit_context.ExitReason == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonMemoryAccess
        {
            println!("Memory access");

            let mem_access_ctx = unsafe { &exit_context.anon_union.MemoryAccess };
            let _status = e.try_mmio_emulation(
                std::ptr::null_mut(),
                &exit_context.VpContext,
                mem_access_ctx,
            ).unwrap();
        } else if exit_context.ExitReason
            == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonX64IoPortAccess
        {
            let io_port_access_ctx = unsafe { &exit_context.anon_union.IoPortAccess };
            let _status = e.try_io_emulation(
                std::ptr::null_mut(),
                &exit_context.VpContext,
                io_port_access_ctx,
            ).unwrap();
        } else if exit_context.ExitReason
            == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonUnrecoverableException
        {
            panic!("Unrecoverable exception");
        } else if exit_context.ExitReason == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonX64Cpuid {
            let cpuid_access = unsafe { exit_context.anon_union.CpuidAccess };
            println!("Got CPUID leaf: {}", cpuid_access.Rax);

            const NUM_REGS: UINT32 = 5;
            let mut reg_names: [WHV_REGISTER_NAME; NUM_REGS as usize] =
                unsafe { std::mem::zeroed() };
            let mut reg_values: [WHV_REGISTER_VALUE; NUM_REGS as usize] =
                unsafe { std::mem::zeroed() };

            reg_names[0] = WHV_REGISTER_NAME::WHvX64RegisterRip;
            reg_names[1] = WHV_REGISTER_NAME::WHvX64RegisterRax;
            reg_names[2] = WHV_REGISTER_NAME::WHvX64RegisterRbx;
            reg_names[3] = WHV_REGISTER_NAME::WHvX64RegisterRcx;
            reg_names[4] = WHV_REGISTER_NAME::WHvX64RegisterRdx;

            reg_values[0].Reg64 = exit_context.VpContext.Rip
                + (exit_context.VpContext.InstructionLengthCr8 & 0xff) as u64;
            reg_values[1].Reg64 = cpuid_access.DefaultResultRax;
            reg_values[2].Reg64 = cpuid_access.DefaultResultRbx;
            reg_values[3].Reg64 = cpuid_access.DefaultResultRcx;
            reg_values[4].Reg64 = cpuid_access.DefaultResultRdx;

            match cpuid_access.Rax {
                1 => {
                    reg_values[3].Reg64 = CPUID_EXT_HYPERVISOR as UINT64;
                }
                _ => {
                    println!("Unknown CPUID leaf: {}", cpuid_access.Rax);
                }
            }

            vp.set_registers(&reg_names, &reg_values).unwrap();
        } else if exit_context.ExitReason == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonX64MsrAccess
        {
            let msr_access = unsafe { exit_context.anon_union.MsrAccess };

            const NUM_REGS: UINT32 = 3;
            let mut reg_names: [WHV_REGISTER_NAME; NUM_REGS as usize] =
                unsafe { std::mem::zeroed() };
            let mut reg_values: [WHV_REGISTER_VALUE; NUM_REGS as usize] =
                unsafe { std::mem::zeroed() };

            reg_names[0] = WHV_REGISTER_NAME::WHvX64RegisterRip;
            reg_names[1] = WHV_REGISTER_NAME::WHvX64RegisterRax;
            reg_names[2] = WHV_REGISTER_NAME::WHvX64RegisterRdx;

            reg_values[0].Reg64 = exit_context.VpContext.Rip
                + (exit_context.VpContext.InstructionLengthCr8 & 0xff) as u64;

            let is_write = (msr_access.AccessInfo & 0x1) != 0;

            if is_write {
                println!(
                    "Got write MSR. Number: {}, Rax: {}, Rdx: {}",
                    msr_access.MsrNumber, msr_access.Rax, msr_access.Rdx
                );
            } else {
                println!("Got read MSR. Number: {}", msr_access.MsrNumber);
            }

            match msr_access.MsrNumber {
                1 => {
                    reg_values[1].Reg64 = 1000;
                    reg_values[2].Reg64 = 1001;
                }
                _ => {
                    println!("Unknown MSR number: {}", msr_access.MsrNumber);
                }
            }

            let mut num_regs_set = NUM_REGS as usize;
            if is_write {
                num_regs_set = 1;
            }

            vp.set_registers(&reg_names[0..num_regs_set], &reg_values[0..num_regs_set])
                .unwrap();
        } else {
            panic!("Unexpected exit type");
        }
    }

    unsafe { VirtualFree(mem_addr, 0, MEM_RELEASE) };
}

struct SampleCallbacks<'a> {
    vp: &'a VirtualProcessor<'a>,
}

impl<'a> EmulatorCallbacks for SampleCallbacks<'a> {
    fn io_port(
        &mut self,
        _context: *mut VOID,
        io_access: &mut WHV_EMULATOR_IO_ACCESS_INFO,
    ) -> HRESULT {
        if io_access.Port == 42 {
            let data = unsafe {
                std::slice::from_raw_parts(
                    &io_access.Data as *const _ as *const u8,
                    io_access.AccessSize as usize,
                )
            };
            io::stdout().write(data).unwrap();
        } else {
            println!("Unsupported IO port");
        }
        S_OK
    }
    fn memory(
        &mut self,
        _context: *mut VOID,
        memory_access: &mut WHV_EMULATOR_MEMORY_ACCESS_INFO,
    ) -> HRESULT {
        println!("memory");
        println!("{:?}", memory_access);
        S_OK
    }
    fn get_virtual_processor_registers(
        &mut self,
        _context: *mut VOID,
        register_names: &[WHV_REGISTER_NAME],
        register_values: &mut [WHV_REGISTER_VALUE],
    ) -> HRESULT {
        self.vp
            .get_registers(register_names, register_values)
            .unwrap();
        S_OK
    }
    fn set_virtual_processor_registers(
        &mut self,
        _context: *mut VOID,
        register_names: &[WHV_REGISTER_NAME],
        register_values: &[WHV_REGISTER_VALUE],
    ) -> HRESULT {
        self.vp
            .set_registers(register_names, register_values)
            .unwrap();
        S_OK
    }
    fn translate_gva_page(
        &mut self,
        _context: *mut VOID,
        gva: WHV_GUEST_VIRTUAL_ADDRESS,
        translate_flags: WHV_TRANSLATE_GVA_FLAGS,
        translation_result: &mut WHV_TRANSLATE_GVA_RESULT_CODE,
        gpa: &mut WHV_GUEST_PHYSICAL_ADDRESS,
    ) -> HRESULT {
        let (translation_result1, gpa1) = self.vp.translate_gva(gva, translate_flags).unwrap();

        *translation_result = translation_result1.ResultCode;
        *gpa = gpa1;
        S_OK
    }
}
