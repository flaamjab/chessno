use std::sync::Arc;

use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;

use crate::logging::debug;

const MAX_EXPECTED_SYNC_PRIMITIVES: usize = 8;

pub struct SyncPool {
    semaphores: SmallVec<[vk::Semaphore; MAX_EXPECTED_SYNC_PRIMITIVES]>,
    fences: SmallVec<[vk::Fence; MAX_EXPECTED_SYNC_PRIMITIVES]>,
}

impl SyncPool {
    pub fn new() -> Self {
        Self {
            semaphores: SmallVec::with_capacity(MAX_EXPECTED_SYNC_PRIMITIVES),
            fences: SmallVec::with_capacity(MAX_EXPECTED_SYNC_PRIMITIVES),
        }
    }

    pub unsafe fn semaphore(&mut self, device: &Arc<DeviceLoader>) -> vk::Semaphore {
        let semaphore_info = vk::SemaphoreCreateInfoBuilder::new();
        let semaphore = device
            .create_semaphore(&semaphore_info, None)
            .expect("failed to create a semaphore");
        self.semaphores.push(semaphore);

        semaphore
    }

    pub unsafe fn fence(&mut self, device: &Arc<DeviceLoader>, signaled: bool) -> vk::Fence {
        let mut fence_info = vk::FenceCreateInfoBuilder::new();
        if signaled {
            fence_info = fence_info.flags(vk::FenceCreateFlags::SIGNALED);
        }

        let fence = device
            .create_fence(&fence_info, None)
            .expect("failed to create a fence");
        self.fences.push(fence);

        fence
    }

    pub unsafe fn destroy_all(&mut self, device: &Arc<DeviceLoader>) {
        for s in &self.semaphores {
            device.destroy_semaphore(*s, None);
        }

        for f in &self.fences {
            device.destroy_fence(*f, None);
        }

        self.fences.clear();
        self.semaphores.clear();
    }
}

impl Drop for SyncPool {
    fn drop(&mut self) {
        debug!("A sync pool would be dropped now");
    }
}
