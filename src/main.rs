extern crate libwhp;

use libwhp::instruction_emulator::*;
use libwhp::*;

fn main() {
    let p: Partition = Partition::new().unwrap();

    let mut property: WHV_PARTITION_PROPERTY = unsafe { std::mem::zeroed() };
    property.ProcessorCount = 1;
    p.set_property(
        WHV_PARTITION_PROPERTY_CODE::WHvPartitionPropertyCodeProcessorCount,
        &property,
    ).unwrap();

    p.setup().unwrap();

    // Replace with an actual mapping
    const SIZE: UINT64 = 1024;
    let source_address = Box::new([0; SIZE as usize]);
    let guest_address: WHV_GUEST_PHYSICAL_ADDRESS = 0;

    p.map_gpa_range(
        source_address.as_ptr() as *const VOID,
        guest_address,
        SIZE,
        WHV_MAP_GPA_RANGE_FLAGS::WHvMapGpaRangeFlagRead,
    ).unwrap();

    let vp = p.create_virtual_processor(0).unwrap();

    // Replace with actual register values
    const NUM_REGS: UINT32 = 1;
    let mut reg_names: [WHV_REGISTER_NAME; NUM_REGS as usize] = unsafe { std::mem::zeroed() };
    let mut reg_values: [WHV_REGISTER_VALUE; NUM_REGS as usize] = unsafe { std::mem::zeroed() };

    reg_names[0] = WHV_REGISTER_NAME::WHvX64RegisterRax;
    reg_values[0].Reg64 = 0;

    vp.set_registers(&reg_names, &reg_values).unwrap();

    loop {
        let exit_context = vp.run().unwrap();
        // Handle exits
        if exit_context.ExitReason == WHV_RUN_VP_EXIT_REASON::WHvRunVpExitReasonX64Halt {
            break;
        }
    }

    // To translate a GVA into a GPA:
    let gva: WHV_GUEST_PHYSICAL_ADDRESS = 0;
    let (_translation_result, _gpa) = vp.translate_gva(
        gva,
        WHV_TRANSLATE_GVA_FLAGS::WHvTranslateGvaFlagValidateRead,
    ).unwrap();

    let mut callbacks = MyCallbacks {};
    let _e = Emulator::new(&mut callbacks).unwrap();
}

struct MyCallbacks {}

impl EmulatorCallbacks for MyCallbacks {
    fn io_port(
        &mut self,
        _context: *mut VOID,
        _io_access: &mut WHV_EMULATOR_IO_ACCESS_INFO,
    ) -> HRESULT {
        S_OK
    }
    fn memory(
        &mut self,
        _context: *mut VOID,
        _memory_access: &mut WHV_EMULATOR_MEMORY_ACCESS_INFO,
    ) -> HRESULT {
        S_OK
    }
    fn get_virtual_processor_registers(
        &mut self,
        _context: *mut VOID,
        _register_names: &[WHV_REGISTER_NAME],
        _register_values: &mut [WHV_REGISTER_VALUE],
    ) -> HRESULT {
        S_OK
    }
    fn set_virtual_processor_registers(
        &mut self,
        _context: *mut VOID,
        _register_names: &[WHV_REGISTER_NAME],
        _register_values: &[WHV_REGISTER_VALUE],
    ) -> HRESULT {
        S_OK
    }
    fn translate_gva_page(
        &mut self,
        _context: *mut VOID,
        _gva: WHV_GUEST_VIRTUAL_ADDRESS,
        _translate_flags: WHV_TRANSLATE_GVA_FLAGS,
        _translation_result: &mut WHV_TRANSLATE_GVA_RESULT_CODE,
        _gpa: &mut WHV_GUEST_PHYSICAL_ADDRESS,
    ) -> HRESULT {
        S_OK
    }
}
