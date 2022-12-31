use std::{
    borrow::Cow,
    ffi::{c_char, c_void, CStr},
    sync::Arc,
};

use ash::{
    extensions::ext::{DebugReport, DebugUtils},
    prelude::VkResult,
    vk::{self, ExtExtension390Fn, Handle},
    Device, Instance,
};
use vma::Allocator;

use chessno::logging::debug;
use chessno::vma;

pub struct Context {
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    device: Device,
    allocator: Allocator,
    swapchain: vk::SwapchainKHR,

    debug: Option<Debug>,
}

struct Debug {
    debug_utils_loader: DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Context {
    pub fn new(window: Option<u64>, handle_type: WindowHandleType, debug: bool) -> Arc<Self> {
        unsafe {
            let entry = ash::Entry::linked();

            let app_name = CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0");

            let mut layer_names = Vec::new();
            let mut extension_names = Vec::new();
            if debug {
                layer_names.push(CStr::from_bytes_with_nul_unchecked(
                    b"VK_LAYER_KHRONOS_validation\0",
                ));
                extension_names.push(DebugUtils::name().as_ptr());
            }

            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let app_info = vk::ApplicationInfo::builder()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 0, 0));

            let create_flags = vk::InstanceCreateFlags::default();
            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names)
                .flags(create_flags);

            let instance = entry.create_instance(&create_info, None).unwrap();
            debug!("Vulkan instance created");

            let maybe_debug = if debug {
                let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback));

                let debug_utils_loader = DebugUtils::new(&entry, &instance);
                debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .map_err(|e| {
                        debug!("Failed to create debug utils messenger");
                    })
                    .map(|m| {
                        Some(Debug {
                            debug_messenger: m,
                            debug_utils_loader,
                        })
                    })
                    .unwrap_or_default()
            } else {
                None
            };

            let physical_device = Self::find_suitable_physical_device(&instance);
            let pd_properties = instance.get_physical_device_properties(physical_device);
            debug!(
                "Using device:\nid: {},\ndevice name: {:?}",
                pd_properties.device_id,
                CStr::from_ptr(pd_properties.device_name.as_ptr() as *const i8),
            );

            let mut queue_family_index = None;
            for (index, queue_family) in instance
                .get_physical_device_queue_family_properties(physical_device)
                .into_iter()
                .enumerate()
            {
                // Not using contains here because reasons
                let required_flags =
                    vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER;
                let supported = (queue_family.queue_flags & required_flags) == required_flags;

                if supported {
                    queue_family_index = Some(index as u32);
                    break;
                }
            }

            let queue_family_index = queue_family_index.unwrap();
            debug!(
                "Suitable queue family with index {} found",
                queue_family_index
            );

            let priorities = [1.0];
            let queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities);

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_info));

            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap();
            debug!("Created Vulkan device");

            let mut allocator = Allocator::new(&instance, &physical_device, &device).unwrap();
            debug!("VMA allocator created");

            let buffer_info = vk::BufferCreateInfo::builder()
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .size(256)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let buffer = device.create_buffer(&buffer_info, None).unwrap();

            debug!("Allocating memory");

            let memory = allocator.allocate_for_buffer(buffer, true).unwrap();
            debug!("Memory allocated");

            allocator.set_memory_data(&memory, b"data");

            debug!("Freeing memory");
            allocator.free_memory(memory);
            debug!("Memory freed");

            debug!("Destroying buffer");
            device.destroy_buffer(buffer, None);
            debug!("Buffer destroyed");

            Arc::new(Self {
                instance,
                physical_device,
                device,
                allocator,
                swapchain: vk::SwapchainKHR::null(),
                debug: maybe_debug,
            })
        }
    }

    unsafe fn find_suitable_physical_device(instance: &Instance) -> vk::PhysicalDevice {
        let pdevices = instance
            .enumerate_physical_devices()
            .expect("physical device error");
        pdevices
            .into_iter()
            .next()
            .expect("couldn't find suitable device.")
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.allocator.destroy();
            debug!("VMA allocator destroyed");

            self.device.destroy_device(None);
            debug!("Device destroyed");

            if let Some(debug) = &self.debug {
                debug
                    .debug_utils_loader
                    .destroy_debug_utils_messenger(debug.debug_messenger, None);
                debug!("Debug utils messenger");
            }

            self.instance.destroy_instance(None);
            debug!("Instance destroyed");

            debug!("Vulkan context dropped");
        }
    }
}

pub enum WindowHandleType {
    X11,
    Wayland,
    Win32,
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    debug!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity, message_type, message_id_name, message_id_number, message,
    );

    vk::FALSE
}
