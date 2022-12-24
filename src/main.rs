use std::borrow::Cow;
use std::ffi::CStr;

use ash::vk;
use chessno::logging::debug;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

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

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity, message_type, message_id_name, message_id_number, message,
    );

    vk::FALSE
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("Ash - Example")
        .with_inner_size(LogicalSize::new(f64::from(800), f64::from(600)))
        .build(&event_loop)
        .unwrap();

    event_loop.run(|event, _, control_flow| {
        control_flow.set_poll();
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                draw();
            }
            _ => (),
        }
    });
}

fn draw() {}
