use erupt::{vk, DeviceLoader};
use smallvec::SmallVec;

use crate::physical_device::PhysicalDevice;

pub struct Swapchain {
    handle: vk::SwapchainKHR,
    image_views: SmallVec<[vk::ImageView; 8]>,
    image_extent: vk::Extent2D,
    image_count: u32,
    framebuffers: Option<SmallVec<[vk::Framebuffer; 8]>>,
}

impl Swapchain {
    pub unsafe fn new(
        device: &DeviceLoader,
        physical_device: &PhysicalDevice,
        surface: vk::SurfaceKHR,
        draw_area_size: &vk::Extent2D,
    ) -> Self {
        // Select image count
        let image_count = select_image_count(physical_device);
        // Find extent
        let image_extent = compute_extent(physical_device, &draw_area_size);

        let swapchain = create_swapchain(
            device,
            physical_device,
            surface,
            image_count,
            &image_extent,
            vk::SwapchainKHR::null(),
        );
        let image_views = create_image_views(device, swapchain, physical_device.surface_format);

        Self {
            handle: swapchain,
            image_views,
            image_count,
            image_extent,
            framebuffers: None,
        }
    }

    pub unsafe fn recreate(
        &mut self,
        device: &DeviceLoader,
        physical_device: &PhysicalDevice,
        surface: vk::SurfaceKHR,
        draw_area_size: &vk::Extent2D,
    ) {
        self.release_dependents(device);

        let new_image_extent = compute_extent(physical_device, draw_area_size);
        let new_swapchain = create_swapchain(
            device,
            physical_device,
            surface,
            self.image_count,
            &new_image_extent,
            self.handle,
        );

        device.destroy_swapchain_khr(self.handle, None);

        self.image_extent = new_image_extent;
        self.image_views =
            create_image_views(device, new_swapchain, physical_device.surface_format);
        self.handle = new_swapchain;
    }

    pub fn handle(&self) -> vk::SwapchainKHR {
        self.handle
    }

    pub unsafe fn framebuffers(
        &mut self,
        device: &DeviceLoader,
        render_pass: vk::RenderPass,
    ) -> &[vk::Framebuffer] {
        match self.framebuffers {
            Some(_) => self.framebuffers.as_ref().unwrap(),
            None => {
                let fbs =
                    create_framebuffers(device, &self.image_views, render_pass, &self.image_extent);
                self.framebuffers = Some(fbs);
                self.framebuffers.as_ref().unwrap()
            }
        }
    }

    pub fn image_extent(&self) -> &vk::Extent2D {
        &self.image_extent
    }

    pub unsafe fn destroy(&mut self, device: &DeviceLoader) {
        self.release_dependents(device);
        device.destroy_swapchain_khr(self.handle, None);
    }

    unsafe fn release_dependents(&mut self, device: &DeviceLoader) {
        if let Some(framebuffers) = &self.framebuffers {
            for fb in framebuffers {
                device.destroy_framebuffer(*fb, None);
            }
            self.framebuffers = None;
        }

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
    render_pass: vk::RenderPass,
    extent: &vk::Extent2D,
) -> SmallVec<[vk::Framebuffer; 8]> {
    image_views
        .iter()
        .map(|image_view| {
            let attachments = vec![*image_view];
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
