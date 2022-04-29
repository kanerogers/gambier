pub mod buffer;
pub mod vulkan_context;

use std::time::Instant;

use nalgebra_glm as glm;
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
    let mut camera_pos = nalgebra_glm::vec3(0., 0., 2.);
    let mut camera_y_rot = 0.;

    let projection = create_projection_matrix();
    let view = update_camera(camera_y_rot, &camera_pos);
    let mut globals = Globals { projection, view };
    let mut last_frame_time = Instant::now();

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
                    let delta_time = (Instant::now() - last_frame_time).as_secs_f32();
                    let displacement = (100 / 1) as f32 * delta_time;
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Key1) => {
                            selected_pipeline = SelectedPipeline::Colored;
                        }
                        Some(VirtualKeyCode::W) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &glm::vec3(0., 0., -displacement),
                                camera_y_rot,
                            );
                            camera_pos += delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::S) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(0., 0., displacement),
                                camera_y_rot,
                            );
                            camera_pos += delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::A) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(displacement, 0., 0.),
                                camera_y_rot,
                            );
                            camera_pos += delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::D) => {
                            let delta = nalgebra_glm::rotate_y_vec3(
                                &nalgebra_glm::vec3(-displacement, 0., 0.),
                                camera_y_rot,
                            );
                            camera_pos += delta;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::Q) => {
                            camera_y_rot -= displacement;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        Some(VirtualKeyCode::E) => {
                            camera_y_rot += displacement;
                            globals.view = update_camera(camera_y_rot, &camera_pos);
                        }
                        _ => {}
                    }
                }
            }

            winit::event::Event::MainEventsCleared => unsafe {
                context.render(&selected_pipeline, &globals);
                last_frame_time = Instant::now();
            },
            _ => {}
        }
    });
}

#[allow(unused)]
fn create_projection_matrix() -> glm::TMat4<f32> {
    let aspect_ratio = 800. / 600.;
    let fov_y = 70_f32.to_radians();
    let f = 1.0 / (fov_y / 2.0).tan();
    let z_near = 0.5;

    #[rustfmt::skip]
    let niagra = nalgebra_glm::mat4(
        f / aspect_ratio, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, 0., -1.0,
        0.0, 0.0, z_near, 0.0,
    );

    let fov_rad = fov_y * 2.0 * std::f32::consts::PI / 360.0;
    let focal_length = 1.0 / (fov_rad / 2.0).tan();

    let n = z_near;
    let x = focal_length / aspect_ratio;
    let y = -focal_length;
    let a = n / (f - n);
    let b = f * a;

    #[rustfmt::skip]
    let vulkan = nalgebra_glm::mat4(
        x, 0.0, 0.0, 0.0,
        0.0, y, 0.0, 0.0,
        0.0, 0.0, a, b, 
        0.0, 0.0, -1.0, 0.0,
    );

    // NOTE: Requires depth testing to be configured as per vkguide - NOT niagra.
    let mut vulkan_guide = glm::perspective(fov_y, aspect_ratio, 0.1, 200.0);
    vulkan_guide.m11 *= -1.; // inverse Y for Vulkan

    // niagra
    // vulkan
    vulkan_guide
}

fn update_camera(camera_y_rot: f32, camera_pos: &nalgebra_glm::Vec3) -> nalgebra_glm::TMat4<f32> {
    let new = nalgebra_glm::rotate_y(
        &nalgebra_glm::translate(&nalgebra_glm::identity(), camera_pos),
        camera_y_rot,
    )
    .try_inverse()
    .unwrap();

    println!("Camera is now: {:?}", new);
    new
}
