use std;
use winapi;
use winapi::um::memoryapi::{VirtualAlloc, VirtualFree};
use winapi::um::winnt::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE};

pub struct VirtualMemory {
    address: *mut winapi::ctypes::c_void,
    size: usize,
}

impl VirtualMemory {
    pub fn new(size: usize) -> VirtualMemory {
        let address = unsafe {
            VirtualAlloc(
                std::ptr::null_mut(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };

        match address as u64 {
            0 => panic!("VirtualAlloc failed"),
            _ => VirtualMemory {
                address: address,
                size: size,
            },
        }
    }

    pub fn as_slice_mut<'a>(&'a mut self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.address as *mut u8, self.size) }
    }

    pub fn as_ptr<'a>(&'a self) -> *const std::os::raw::c_void {
        self.address as *const std::os::raw::c_void
    }

    pub fn get_size(&self) -> usize {
        self.size
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        unsafe { VirtualFree(self.address, 0, MEM_RELEASE) };
    }
}
