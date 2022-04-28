pub mod buffer;
pub mod vulkan_context;

use vulkan_context::{SelectedPipeline, VulkanContext};
use winit::{
    event::{VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = VulkanContext::new(&window);
    let mut selected_pipeline = SelectedPipeline::Colored;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if input.state == winit::event::ElementState::Released {
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Key1) => {
                            selected_pipeline = SelectedPipeline::Colored;
                        }
                        _ => {}
                    }
                }
            }

            winit::event::Event::MainEventsCleared => unsafe {
                context.render(&selected_pipeline);
            },
            _ => {}
        }
    });
}
