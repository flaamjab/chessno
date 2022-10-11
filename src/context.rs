use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

use erupt::{cstr, utils::surface, vk, DeviceLoader, EntryLoader, InstanceLoader};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::logging::{debug, info};
use crate::physical_device::PhysicalDevice;
use crate::swapchain::Swapchain;
use crate::validation;

const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");

pub struct Context {
    pub swapchain: Swapchain,
    pub queues: Queues,
    pub surface: vk::SurfaceKHR,
    pub device: Arc<DeviceLoader>,
    pub physical_device: PhysicalDevice,
    pub instance: Arc<InstanceLoader>,
    pub entry: Arc<EntryLoader>,
}

pub struct Queues {
    pub graphics: vk::Queue,
}

impl Context {
    pub fn new(window: &Window, app_name: &str, engine_name: &str) -> Self {
        unsafe {
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

            let instance =
                new_instance(&entry, app_name, engine_name, &instance_extensions, &layers);
            let surface = surface::create_surface(&instance, &window, None)
                .expect("failed to create a surface");

            #[cfg(all(debug_assertions, not(target_os = "android")))]
            validation::init(&instance);

            let device_extensions = [vk::KHR_SWAPCHAIN_EXTENSION_NAME];
            let physical_device = PhysicalDevice::new(&instance, surface, &device_extensions);

            #[cfg(debug_assertions)]
            let device =
                new_logical_device(&instance, &physical_device, &device_extensions, &layers);

            let graphics_queue =
                device.get_device_queue(physical_device.queue_families.graphics, 0);
            let queues = Queues {
                graphics: graphics_queue,
            };

            info!(
                "Using physical device: {:?}",
                CStr::from_ptr(physical_device.properties.device_name.as_ptr())
            );

            let PhysicalSize { width, height } = window.inner_size();
            let draw_area_size = vk::Extent2D { width, height };
            let swapchain = Swapchain::new(&device, &physical_device, surface, &draw_area_size);

            Self {
                queues,
                surface,
                device,
                physical_device,
                instance,
                entry,
                swapchain,
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        debug!("Dropping Vulkan context");
        unsafe {
            #[cfg(all(debug_assertions, not(target_os = "android")))]
            validation::deinit(&self.instance);

            self.swapchain.destroy(&self.device);

            self.device.destroy_device(None);

            self.instance.destroy_surface_khr(self.surface, None);
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

unsafe fn new_logical_device(
    instance: &InstanceLoader,
    physical_device: &PhysicalDevice,
    device_extensions: &[*const c_char],
    device_layers: &[*const c_char],
) -> Arc<DeviceLoader> {
    let queue_infos = vec![vk::DeviceQueueCreateInfoBuilder::new()
        .queue_family_index(physical_device.queue_families.graphics)
        .queue_priorities(&[1.0])];
    let features = vk::PhysicalDeviceFeaturesBuilder::new();

    let device_info = vk::DeviceCreateInfoBuilder::new()
        .queue_create_infos(&queue_infos)
        .enabled_features(&features)
        .enabled_extension_names(&device_extensions)
        .enabled_layer_names(&device_layers);

    Arc::new(DeviceLoader::new(&instance, physical_device.handle, &device_info).unwrap())
}
