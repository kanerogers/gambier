pub mod buffer;
mod camera;
mod camera_controller;
pub mod frame;
pub mod image;
pub mod memory;
pub mod model;
pub mod swapchain;
pub mod sync_structures;
pub mod texture;
mod timer;
pub mod vertex;
pub mod vulkan_context;

use ash::vk;
use camera_controller::CameraController;
use glm::{vec3, Vec3};
use model::{import_models, ModelContext};
use nalgebra_glm as glm;

use timer::Timer;
use vulkan_context::{Globals, VulkanContext};
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_cursor_grab(true).unwrap();
    window.set_cursor_visible(false);
    let gpu_type = get_gpu_type();
    let mut vulkan_context = VulkanContext::new(&window, gpu_type);
    let mut camera_controller = CameraController::default();

    let projection = create_projection_matrix();
    let view = camera_controller.view();
    let mut globals = Globals { projection, view };
    let mut model_context = import_models(&vulkan_context);
    let resolution = 50;
    let mut cubes = create_cubes(&mut model_context, resolution);
    let mut timer = Timer::default();

    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        winit::event::Event::DeviceEvent { event, .. } => {
            camera_controller.input(event, timer.delta());
        }
        winit::event::Event::MainEventsCleared => unsafe {
            globals.view = camera_controller.view();
            vulkan_context.render(&model_context, &mut globals);
            tick(&mut model_context, timer.time(), &mut cubes);
            timer.tick();
        },
        _ => {}
    });
}

fn tick(model_context: &mut ModelContext, elapsed_time: f32, cubes: &mut Vec<Vec3>) {
    let models = &mut model_context.models;
    let materials = &mut model_context.materials;

    let scale = models.len() as f32 / 2.;
    for (n, model) in models.iter_mut().enumerate() {
        let translation = &mut cubes[n];
        translation.y = f32::sin(std::f32::consts::PI * (translation.x + elapsed_time));
        let transform = glm::translate(&glm::identity(), &translation);

        let scaling = 1. / scale;
        model.transform = glm::scale(&transform, &vec3(scaling, scaling, scaling));
        let material = &mut materials[n];

        let colour = translation.clone() * 0.5 + vec3(0.5, 0.5, 0.5);
        material.base_color_factor = glm::clamp(&glm::vec3_to_vec4(&colour), 0., 1.);
    }
}

fn create_cubes(model_context: &mut ModelContext, resolution: usize) -> Vec<Vec3> {
    let models = &mut model_context.models;
    let cube0 = models.pop().unwrap();
    models.clear();

    let meshes = &mut model_context.meshes;
    let mesh = meshes.get(cube0.mesh).cloned().unwrap();

    let materials = &mut model_context.materials;
    let material = materials.pop().unwrap();
    materials.clear();

    let mut cubes = Vec::new();
    let scale = resolution as f32 / 2.;

    for n in 0..resolution {
        let mut c0 = cube0.clone();

        let x = (n as f32 + 0.5) / scale - 1.;
        let y = x * x * x;
        let c0_translation = vec3(x, y, 0.0);

        let transform = glm::translate(&glm::identity(), &c0_translation);

        let scaling = 1. / scale;
        c0.transform = glm::scale(&transform, &vec3(scaling, scaling, scaling));

        let mut mesh = mesh.clone();
        let mut material = material.clone();

        let colour = c0_translation.clone() * 0.5 + vec3(0.5, 0.5, 0.5);
        material.base_color_factor = glm::vec3_to_vec4(&colour);
        materials.push(material);

        mesh.primitives[0].material_id = n as _;
        c0.mesh = meshes.alloc(mesh);

        models.push(c0);
        cubes.push(c0_translation);
    }

    cubes
}

fn get_gpu_type() -> vk::PhysicalDeviceType {
    let mut args = std::env::args();
    if args.nth(1) == Some("integrated".to_string()) {
        return vk::PhysicalDeviceType::INTEGRATED_GPU;
    }

    return vk::PhysicalDeviceType::DISCRETE_GPU;
}

#[allow(unused)]
fn create_projection_matrix() -> glm::TMat4<f32> {
    let aspect_ratio = 800. / 600.;
    let fov_y = 70_f32.to_radians();
    let f = 1.0 / (fov_y / 2.0).tan();
    let z_near = 0.1;

    let fov_rad = fov_y * 2.0 * std::f32::consts::PI / 360.0;
    let focal_length = 1.0 / (fov_rad / 2.0).tan();

    let n = z_near;
    let x = focal_length / aspect_ratio;
    let y = -focal_length;
    let a = n / (f - n);
    let b = f * a;

    let mut vulkan_guide = glm::infinite_perspective_rh_zo(aspect_ratio, fov_y, z_near);
    vulkan_guide.m22 *= -1.; // inverse Y for Vulkan

    vulkan_guide
}
