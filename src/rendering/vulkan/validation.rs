use std::ffi::{c_void, CStr};
use std::sync::Arc;

use erupt::{vk, InstanceLoader};

use crate::logging::debug;

static mut DEBUG_MESSENGER: vk::DebugUtilsMessengerEXT = vk::DebugUtilsMessengerEXT::null();

pub unsafe fn init(instance: &Arc<InstanceLoader>) {
    if DEBUG_MESSENGER.is_null() {
        DEBUG_MESSENGER = setup_debug_messenger(instance);
    } else {
        panic!("debug messenger is already initialized, remove duplicated call to init");
    }
}

pub unsafe fn deinit(instance: &Arc<InstanceLoader>) {
    if !DEBUG_MESSENGER.is_null() {
        instance.destroy_debug_utils_messenger_ext(DEBUG_MESSENGER, None);
    }
}

unsafe extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagBitsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    debug!(
        "{}",
        CStr::from_ptr((*p_callback_data).p_message).to_string_lossy()
    );

    vk::FALSE
}

unsafe fn setup_debug_messenger(instance: &Arc<InstanceLoader>) -> vk::DebugUtilsMessengerEXT {
    let messenger_info = vk::DebugUtilsMessengerCreateInfoEXTBuilder::new()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE_EXT
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING_EXT
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR_EXT,
        )
        .message_type(
            // vk::DebugUtilsMessageTypeFlagsEXT::GENERAL_EXT |
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION_EXT
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE_EXT,
        )
        .pfn_user_callback(Some(debug_callback));

    instance
        .create_debug_utils_messenger_ext(&messenger_info, None)
        .expect("Failed to setup debug messenger")
}
