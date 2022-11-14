use std::{
    cell::{Ref, RefCell},
    ops::Deref,
};

use erupt::{utils::VulkanResult, vk, DeviceLoader, InstanceLoader};
use smallvec::SmallVec;

use crate::logging::debug;
use crate::rendering::vulkan::memory;
use crate::rendering::vulkan::physical_device::PhysicalDevice;

pub struct Swapchain {
    handle: vk::SwapchainKHR,
    surface: vk::SurfaceKHR,
    present_queue: vk::Queue,
    depth_buffer: DepthBuffer,
    image_views: SmallVec<[vk::ImageView; 8]>,
    image_extent: vk::Extent2D,
    image_count: u32,
    framebuffers: RefCell<Option<SmallVec<[vk::Framebuffer; 8]>>>,
}

impl Swapchain {
    pub fn new(
        device: &DeviceLoader,
        physical_device: &PhysicalDevice,
        present_queue: vk::Queue,
        surface: vk::SurfaceKHR,
        surface_size: &vk::Extent2D,
    ) -> Self {
        let image_count = select_image_count(&physical_device);
        let image_extent = compute_extent(&physical_device, &surface_size);

        unsafe {
            let swapchain = create_swapchain(
                &device,
                &physical_device,
                surface,
                image_count,
                &image_extent,
                vk::SwapchainKHR::null(),
            );
            let image_views =
                create_image_views(&device, swapchain, physical_device.surface_format);

            let depth_buffer = DepthBuffer::new(
                device,
                physical_device,
                physical_device.depth_format,
                &image_extent,
            );

            Self {
                handle: swapchain,
                surface,
                depth_buffer,
                present_queue,
                image_views,
                image_count,
                image_extent,
                framebuffers: RefCell::new(None),
            }
        }
    }

    pub fn acquire_image(
        &mut self,
        device: &DeviceLoader,
        in_flight_fence: vk::Fence,
        image_available_semaphore: vk::Semaphore,
    ) -> Option<u32> {
        unsafe {
            device
                .wait_for_fences(&[in_flight_fence], true, u64::MAX)
                .unwrap();

            match device.acquire_next_image_khr(
                self.handle,
                u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            ) {
                VulkanResult {
                    raw: vk::Result::SUCCESS,
                    value: Some(image),
                } => Some(image),
                VulkanResult {
                    raw: vk::Result::ERROR_OUT_OF_DATE_KHR,
                    ..
                } => None,
                _ => {
                    panic!("failed to acquire image from swapchain, aborting...");
                }
            }
        }
    }

    pub unsafe fn recreate(
        &mut self,
        instance: &InstanceLoader,
        device: &DeviceLoader,
        physical_device: &PhysicalDevice,
        surface: Option<vk::SurfaceKHR>,
        surface_size: &vk::Extent2D,
    ) {
        self.release_dependents(device);

        let surface = if let Some(surface) = surface {
            surface
        } else {
            self.surface
        };

        let new_image_extent = compute_extent(physical_device, surface_size);
        let new_swapchain = create_swapchain(
            device,
            physical_device,
            surface,
            self.image_count,
            &new_image_extent,
            self.handle,
        );
        let new_depth_buffer = DepthBuffer::new(
            device,
            physical_device,
            physical_device.depth_format,
            &new_image_extent,
        );

        debug!("Destroying old swapchain");
        device.destroy_swapchain_khr(self.handle, None);

        if self.surface != surface {
            instance.destroy_surface_khr(self.surface, None);
            self.surface = surface;
        }

        self.image_extent = new_image_extent;
        self.image_views =
            create_image_views(device, new_swapchain, physical_device.surface_format);
        self.depth_buffer = new_depth_buffer;
        self.handle = new_swapchain;

        debug!("Swapchain recreated successfully");
    }

    pub fn handle(&self) -> vk::SwapchainKHR {
        self.handle
    }

    pub fn queue(&self) -> vk::Queue {
        self.present_queue
    }

    pub unsafe fn framebuffers(
        &self,
        device: &DeviceLoader,
        render_pass: vk::RenderPass,
    ) -> Ref<[vk::Framebuffer]> {
        if self.framebuffers.borrow().is_none() {
            let fbs = create_framebuffers(
                device,
                &self.image_views,
                self.depth_buffer.image_view,
                render_pass,
                &self.image_extent,
            );
            self.framebuffers.replace(Some(fbs));
        }

        Ref::map(self.framebuffers.borrow(), |o| o.as_deref().unwrap())
    }

    pub fn image_dimensions(&self) -> &vk::Extent2D {
        &self.image_extent
    }

    pub unsafe fn destroy(&mut self, device: &DeviceLoader, instance: &InstanceLoader) {
        self.release_dependents(device);
        device.destroy_swapchain_khr(self.handle, None);
        instance.destroy_surface_khr(self.surface, None);

        self.handle = vk::SwapchainKHR::null();
        self.surface = vk::SurfaceKHR::null();
    }

    unsafe fn release_dependents(&mut self, device: &DeviceLoader) {
        self.depth_buffer.destroy(device);

        if let Some(framebuffers) = self.framebuffers.borrow().deref() {
            for fb in framebuffers {
                device.destroy_framebuffer(*fb, None);
            }
        }

        self.framebuffers.take();

        for iv in &self.image_views {
            device.destroy_image_view(*iv, None);
        }
    }
}

fn select_image_count(physical_device: &PhysicalDevice) -> u32 {
    let min_image_count = physical_device.surface_capabilities.min_image_count;
    let max_image_count = physical_device.surface_capabilities.max_image_count;

    let mut image_count = min_image_count + 1;
    if max_image_count > 0 && image_count > max_image_count {
        image_count = max_image_count;
    }

    image_count
}

fn compute_extent(physical_device: &PhysicalDevice, draw_area_size: &vk::Extent2D) -> vk::Extent2D {
    match physical_device.surface_capabilities.current_extent {
        vk::Extent2D {
            width: u32::MAX,
            height: u32::MAX,
        } => *draw_area_size,
        normal => normal,
    }
}

unsafe fn create_swapchain(
    device: &DeviceLoader,
    physical_device: &PhysicalDevice,
    surface: vk::SurfaceKHR,
    image_count: u32,
    image_extent: &vk::Extent2D,
    old_swapchain: vk::SwapchainKHR,
) -> vk::SwapchainKHR {
    let info = vk::SwapchainCreateInfoKHRBuilder::new()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(physical_device.surface_format.format)
        .image_color_space(physical_device.surface_format.color_space)
        .image_extent(*image_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(physical_device.surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
        .present_mode(physical_device.present_mode)
        .clipped(true)
        .old_swapchain(old_swapchain);

    let swapchain = device
        .create_swapchain_khr(&info, None)
        .expect("failed to create swapchain");

    swapchain
}

unsafe fn create_image_views(
    device: &DeviceLoader,
    swapchain: vk::SwapchainKHR,
    format: vk::SurfaceFormatKHR,
) -> SmallVec<[vk::ImageView; 8]> {
    let swapchain_images = device.get_swapchain_images_khr(swapchain, None).unwrap();

    swapchain_images
        .iter()
        .map(|swapchain_image| {
            let image_view_info = vk::ImageViewCreateInfoBuilder::new()
                .image(*swapchain_image)
                .view_type(vk::ImageViewType::_2D)
                .format(format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(
                    vk::ImageSubresourceRangeBuilder::new()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                );
            device.create_image_view(&image_view_info, None).unwrap()
        })
        .collect()
}

unsafe fn create_framebuffers(
    device: &DeviceLoader,
    image_views: &[vk::ImageView],
    depth_image_view: vk::ImageView,
    render_pass: vk::RenderPass,
    extent: &vk::Extent2D,
) -> SmallVec<[vk::Framebuffer; 8]> {
    image_views
        .iter()
        .map(|image_view| {
            let attachments = [*image_view, depth_image_view];
            let framebuffer_info = vk::FramebufferCreateInfoBuilder::new()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

            device.create_framebuffer(&framebuffer_info, None).unwrap()
        })
        .collect()
}

pub struct DepthBuffer {
    pub memory: vk::DeviceMemory,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
}

impl DepthBuffer {
    pub fn destroy(&self, device: &DeviceLoader) {
        unsafe {
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}

impl DepthBuffer {
    pub fn new(
        device: &DeviceLoader,
        physical_device: &PhysicalDevice,
        format: vk::Format,
        extent: &vk::Extent2D,
    ) -> Self {
        unsafe {
            let (depth_buffer_image, depth_buffer_mem) =
                memory::create_depth_buffer(device, physical_device, format, &extent);
            let depth_buffer_view = memory::create_image_view(
                device,
                depth_buffer_image,
                format,
                vk::ImageAspectFlags::DEPTH,
            );

            Self {
                memory: depth_buffer_mem,
                image: depth_buffer_image,
                image_view: depth_buffer_view,
            }
        }
    }
}
