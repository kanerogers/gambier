pub mod buffer;
pub mod vulkan_context;

use vulkan_context::{Globals, SelectedPipeline, VulkanContext};
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
    let mut camera_pos = nalgebra_glm::vec3(0., 0., 0.);
    let mut camera_y_rot = 0.;

    let mut projection =
        nalgebra_glm::reversed_infinite_perspective_rh_zo(800. / 600., 70_f32.to_radians(), 0.5);
    projection.m11 = -1.;
    let view = update_camera(camera_y_rot, &camera_pos);
    let mut globals = Globals { projection, view };

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
                if input.state == winit::event::ElementState::Pressed {
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Key1) => {
                            selected_pipeline = SelectedPipeline::Colored;
                        }
                        Some(VirtualKeyCode::W) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(0., 0., 0.1),
                                -camera_y_rot,
                            );
                            camera_pos += delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::S) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(0., 0., 0.1),
                                -camera_y_rot,
                            );
                            camera_pos -= delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::A) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(0.1, 0., 0.),
                                -camera_y_rot,
                            );
                            camera_pos -= delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::D) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(0.1, 0., 0.),
                                -camera_y_rot,
                            );
                            camera_pos += delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::Q) => {
                            camera_y_rot += 0.01;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::E) => {
                            camera_y_rot -= 0.01;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        _ => {}
                    }
                }
            }

            winit::event::Event::MainEventsCleared => unsafe {
                context.render(&selected_pipeline, &globals);
            },
            _ => {}
        }
    });
}

fn update_camera(camera_y_rot: f32, camera_pos: &nalgebra_glm::Vec3) -> nalgebra_glm::TMat4<f32> {
    nalgebra_glm::translate(
        &nalgebra_glm::rotate_y(&nalgebra_glm::identity(), camera_y_rot),
        camera_pos,
    )
}
