use std::borrow::Cow;
use std::ffi::CStr;

use ash::vk;
use context::Context;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod context;

fn main() {
    env_logger::init();

    let context = Context::new(None, context::WindowHandleType::Win32, true);
    // let event_loop = EventLoop::new();
    // let _window = WindowBuilder::new()
    //     .with_title("Ash - Example")
    //     .with_inner_size(LogicalSize::new(f64::from(800), f64::from(600)))
    //     .build(&event_loop)
    //     .unwrap();

    // event_loop.run(|event, _, control_flow| {
    //     control_flow.set_poll();
    //     match event {
    //         Event::WindowEvent {
    //             event:
    //                 WindowEvent::CloseRequested
    //                 | WindowEvent::KeyboardInput {
    //                     input:
    //                         KeyboardInput {
    //                             state: ElementState::Pressed,
    //                             virtual_keycode: Some(VirtualKeyCode::Escape),
    //                             ..
    //                         },
    //                     ..
    //                 },
    //             ..
    //         } => *control_flow = ControlFlow::Exit,
    //         Event::MainEventsCleared => {
    //             draw();
    //         }
    //         _ => (),
    //     }
    // });
}

fn draw() {}
