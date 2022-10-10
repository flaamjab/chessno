use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

use erupt::{cstr, utils::surface, vk, DeviceLoader, EntryLoader, InstanceLoader};
use winit::window::Window;

use crate::logging::{debug, info};
use crate::validation;

const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");

pub struct Context {
    pub queues: Queues,
    pub surface: vk::SurfaceKHR,
    pub device: Arc<DeviceLoader>,
    pub physical_device: PhysicalDevice,
    pub instance: Arc<InstanceLoader>,
    pub entry: Arc<EntryLoader>,
}

#[derive(Clone)]
pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub queue_families: QueueFamilies,
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub properties: vk::PhysicalDeviceProperties,
}

#[derive(Clone)]
pub struct QueueFamilies {
    pub graphics: u32,
}

pub struct Queues {
    pub graphics: vk::Queue,
}

impl Context {
    pub unsafe fn new(window: &Window, app_name: &str, engine_name: &str) -> Self {
        let entry = Arc::new(EntryLoader::new().expect("Could locate Vulkan on this device"));
        info!(
            "Initializing Vulkan instance {}.{}.{}",
            vk::api_version_major(entry.instance_version()),
            vk::api_version_minor(entry.instance_version()),
            vk::api_version_patch(entry.instance_version())
        );

        let mut instance_extensions = surface::enumerate_required_extensions(window)
            .expect("failed to get required surface extensions");

        let mut layers = Vec::new();
        #[cfg(all(debug_assertions, not(target_os = "android")))]
        {
            instance_extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION_NAME);
            layers.push(LAYER_KHRONOS_VALIDATION);
        }

        let instance = new_instance(&entry, app_name, engine_name, &instance_extensions, &layers);
        let surface =
            surface::create_surface(&instance, &window, None).expect("failed to create a surface");

        #[cfg(all(debug_assertions, not(target_os = "android")))]
        validation::init(&instance);

        let device_extensions = [vk::KHR_SWAPCHAIN_EXTENSION_NAME];
        let physical_device = select_physical_device(&instance, surface, &device_extensions);

        #[cfg(debug_assertions)]
        let device = new_logical_device(&instance, &physical_device, &device_extensions, &layers);

        let graphics_queue = device.get_device_queue(physical_device.queue_families.graphics, 0);
        let queues = Queues {
            graphics: graphics_queue,
        };

        info!(
            "Using physical device: {:?}",
            CStr::from_ptr(physical_device.properties.device_name.as_ptr())
        );

        Self {
            queues,
            surface,
            device,
            physical_device,
            instance,
            entry,
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        debug!("Dropping Vulkan context");
        unsafe {
            #[cfg(all(debug_assertions, not(target_os = "android")))]
            validation::deinit(&self.instance);

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe fn new_instance(
    entry: &EntryLoader,
    app_name: &str,
    engine_name: &str,
    required_extensions: &[*const c_char],
    required_layers: &[*const c_char],
) -> Arc<InstanceLoader> {
    let app_name = CString::new(app_name).unwrap();
    let engine_name = CString::new(engine_name).unwrap();
    let app_info = vk::ApplicationInfoBuilder::new()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::make_api_version(0, 1, 0, 0));

    let instance_info = vk::InstanceCreateInfoBuilder::new()
        .application_info(&app_info)
        .enabled_extension_names(&required_extensions)
        .enabled_layer_names(&required_layers);

    Arc::new(InstanceLoader::new(&entry, &instance_info).expect("failed to create instance"))
}

unsafe fn select_physical_device(
    instance: &InstanceLoader,
    surface: vk::SurfaceKHR,
    required_device_extensions: &[*const c_char],
) -> PhysicalDevice {
    instance
        .enumerate_physical_devices(None)
        .unwrap()
        .into_iter()
        .filter_map(|physical_device| {
            let queue_families = match instance
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
                Some(queue_family) => QueueFamilies {
                    graphics: queue_family as u32,
                },
                None => return None,
            };

            let formats = instance
                .get_physical_device_surface_formats_khr(physical_device, surface, None)
                .unwrap();

            let format = match formats
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
            let device_extensions_supported =
                required_device_extensions.iter().all(|device_extension| {
                    let device_extension = CStr::from_ptr(*device_extension);

                    supported_device_extensions.iter().any(|properties| {
                        CStr::from_ptr(properties.extension_name.as_ptr()) == device_extension
                    })
                });

            if !device_extensions_supported {
                return None;
            }

            let device_properties = instance.get_physical_device_properties(physical_device);

            Some(PhysicalDevice {
                handle: physical_device,
                queue_families,
                surface_format: format,
                present_mode,
                properties: device_properties,
            })
        })
        .max_by_key(
            |PhysicalDevice { properties, .. }| match properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 2,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                _ => 0,
            },
        )
        .expect("No suitable physical device found")
}

unsafe fn new_logical_device(
    instance: &InstanceLoader,
    physical_device: &PhysicalDevice,
    device_extensions: &[*const c_char],
    device_layers: &[*const c_char],
) -> Arc<DeviceLoader> {
    let queue_info = vec![vk::DeviceQueueCreateInfoBuilder::new()
        .queue_family_index(physical_device.queue_families.graphics)
        .queue_priorities(&[1.0])];
    let features = vk::PhysicalDeviceFeaturesBuilder::new();

    let device_info = vk::DeviceCreateInfoBuilder::new()
        .queue_create_infos(&queue_info)
        .enabled_features(&features)
        .enabled_extension_names(&device_extensions)
        .enabled_layer_names(&device_layers);

    Arc::new(DeviceLoader::new(&instance, physical_device.handle, &device_info).unwrap())
}
