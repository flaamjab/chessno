use std::ffi::c_void;

use ash::vk;

#[repr(transparent)]
pub struct VmaAllocatorHandleDummy;
#[repr(transparent)]
pub struct VmaMemoryHandleDummy;

pub type VmaAllocatorHandle = *const VmaAllocatorHandleDummy;
pub type VmaMemoryHandle = *const VmaMemoryHandleDummy;

#[link(name = "vma")]
extern "C" {
    pub fn create_allocator(
        instance: u64,
        physical_device: u64,
        device: u64,
        allocator: *const VmaAllocatorHandle,
    ) -> vk::Result;

    pub fn allocate_memory_for_buffer(
        allocator: VmaAllocatorHandle,
        buffer_object: u64,
        host_visible: bool,
        allocation: *const VmaMemoryHandle,
    ) -> vk::Result;

    pub fn free_memory(allocator: VmaAllocatorHandle, memory: VmaMemoryHandle);

    pub fn set_memory_data(
        allocator: VmaAllocatorHandle,
        memory: VmaMemoryHandle,
        data_in: *const c_void,
        size: usize,
    );

    pub fn destroy_allocator(allocator: VmaAllocatorHandle);
}
