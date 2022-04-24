pub mod vulkan_context;

use vulkan_context::VulkanContext;
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = VulkanContext::new(&window);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            winit::event::Event::MainEventsCleared => unsafe {
                context.render();
            },
            _ => {}
        }
    });
}
