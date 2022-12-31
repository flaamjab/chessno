use std::ffi::{c_char, c_void};
use std::ptr;

use ash::prelude::VkResult;
use ash::vk::{self, Handle};

type VmaHandle = *const ();

pub struct Allocator(VmaHandle);

impl Allocator {
    pub fn new(
        instance: &ash::Instance,
        physical_device: &ash::vk::PhysicalDevice,
        device: &ash::Device,
    ) -> VkResult<Allocator> {
        let mut handle = std::ptr::null();
        unsafe {
            let result = create_allocator(
                instance.handle().as_raw(),
                physical_device.as_raw(),
                device.handle().as_raw(),
                ptr::addr_of_mut!(handle),
            );
            result.result_with_success(Self(handle))
        }
    }

    pub fn allocate_for_buffer(
        &mut self,
        buffer: vk::Buffer,
        host_visible: bool,
    ) -> VkResult<Memory> {
        let mut handle = std::ptr::null();
        unsafe {
            let result = allocate_memory_for_buffer(
                self.0,
                buffer.as_raw(),
                host_visible,
                ptr::addr_of_mut!(handle),
            );
            result.result_with_success(Memory(handle))
        }
    }

    pub fn set_memory_data(&mut self, memory: &Memory, data: &[u8]) {
        unsafe { set_memory_data(self.0, memory.0, data.as_ptr() as *const c_void, data.len()) }
    }

    pub fn free_memory(&mut self, memory: Memory) {
        unsafe {
            free_memory(self.0, memory.0);
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            if !self.0.is_null() {
                destroy_allocator(self.0);
                self.0 = ptr::null();
            }
        }
    }
}

pub struct Memory(VmaHandle);

#[link(name = "vma")]
extern "C" {
    fn create_allocator(
        instance: u64,
        physical_device: u64,
        device: u64,
        allocator: *mut VmaHandle,
    ) -> vk::Result;

    fn allocate_memory_for_buffer(
        allocator: VmaHandle,
        buffer_object: u64,
        host_visible: bool,
        allocation: *mut VmaHandle,
    ) -> vk::Result;

    fn free_memory(allocator: VmaHandle, memory: VmaHandle);

    fn set_memory_data(
        allocator: VmaHandle,
        memory: VmaHandle,
        data_in: *const c_void,
        size: usize,
    );

    fn destroy_allocator(allocator: VmaHandle);
}
