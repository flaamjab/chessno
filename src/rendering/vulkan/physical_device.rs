use std::ffi::CStr;
use std::os::raw::c_char;

use erupt::{vk, InstanceLoader};

#[derive(Clone)]
pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub graphics_queue_family: u32,
    pub depth_format: vk::Format,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
}

#[derive(Clone)]
pub struct SurfaceProperties {
    capabilities: vk::SurfaceCapabilitiesKHR,
    format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
}

impl PhysicalDevice {
    pub unsafe fn new(
        instance: &InstanceLoader,
        surface: vk::SurfaceKHR,
        required_extensions: &[*const c_char],
    ) -> PhysicalDevice {
        instance
            .enumerate_physical_devices(None)
            .unwrap()
            .into_iter()
            .filter_map(|physical_device| {
                let graphics_queue_family = match instance
                    .get_physical_device_queue_family_properties(physical_device, None)
                    .into_iter()
                    .enumerate()
                    .position(|(i, queue_family_properties)| {
                        queue_family_properties
                            .queue_flags
                            .contains(vk::QueueFlags::GRAPHICS)
                            && instance
                                .get_physical_device_surface_support_khr(
                                    physical_device,
                                    i as u32,
                                    surface,
                                )
                                .unwrap()
                    }) {
                    Some(queue_family) => queue_family as u32,
                    None => return None,
                };

                let formats = instance
                    .get_physical_device_surface_formats_khr(physical_device, surface, None)
                    .unwrap();

                let surface_format = match formats
                    .iter()
                    .find(|surface_format| {
                        (surface_format.format == vk::Format::B8G8R8A8_SRGB
                            || surface_format.format == vk::Format::R8G8B8A8_SRGB)
                            && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR_KHR
                    })
                    .or_else(|| formats.get(0))
                {
                    Some(surface_format) => *surface_format,
                    None => return None,
                };

                let present_mode = instance
                    .get_physical_device_surface_present_modes_khr(physical_device, surface, None)
                    .unwrap()
                    .into_iter()
                    .find(|present_mode| present_mode == &vk::PresentModeKHR::MAILBOX_KHR)
                    .unwrap_or(vk::PresentModeKHR::FIFO_KHR);

                let supported_device_extensions = instance
                    .enumerate_device_extension_properties(physical_device, None, None)
                    .unwrap();
                let required_extensions_supported =
                    required_extensions.iter().all(|device_extension| {
                        let device_extension = CStr::from_ptr(*device_extension);

                        supported_device_extensions.iter().any(|properties| {
                            CStr::from_ptr(properties.extension_name.as_ptr()) == device_extension
                        })
                    });

                let features = instance.get_physical_device_features(physical_device);

                if !required_extensions_supported || features.sampler_anisotropy == 0 {
                    return None;
                }

                let properties = instance.get_physical_device_properties(physical_device);

                let surface_capabilities = instance
                    .get_physical_device_surface_capabilities_khr(physical_device, surface)
                    .expect("failed to query surface capabilities");

                let depth_format = find_depth_format(&instance, physical_device)
                    .expect("failed to find supported depth format");

                let memory_properties =
                    instance.get_physical_device_memory_properties(physical_device);

                Some(PhysicalDevice {
                    handle: physical_device,
                    graphics_queue_family,
                    surface_format,
                    depth_format,
                    surface_capabilities,
                    memory_properties,
                    present_mode,
                    properties,
                    features,
                })
            })
            .max_by_key(
                |PhysicalDevice { properties, .. }| match properties.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 2,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                    _ => 0,
                },
            )
            .expect("a Vulkan supported device and OS are required to run")
    }
}

pub unsafe fn find_depth_format(
    instance: &InstanceLoader,
    physical_device: vk::PhysicalDevice,
) -> Option<vk::Format> {
    find_supported_format(
        instance,
        physical_device,
        &[
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ],
        vk::ImageTiling::OPTIMAL,
        vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
    )
}

unsafe fn find_supported_format(
    instance: &InstanceLoader,
    physical_device: vk::PhysicalDevice,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Option<vk::Format> {
    candidates
        .iter()
        .find(|&format| {
            let props = instance.get_physical_device_format_properties(physical_device, *format);

            let mut format_suitable = false;
            match tiling {
                vk::ImageTiling::LINEAR => {
                    if (props.linear_tiling_features & features) == features {
                        format_suitable = true;
                    }
                }
                vk::ImageTiling::OPTIMAL => {
                    if (props.optimal_tiling_features & features) == features {
                        format_suitable = true;
                    }
                }
                _ => {}
            }

            format_suitable
        })
        .map(|&f| f)
}
