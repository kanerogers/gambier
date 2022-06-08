use nalgebra_glm::{Mat4x4, Vec3, Vec4};
use winit::event::{DeviceEvent, ElementState, VirtualKeyCode};

use crate::camera::Camera;

#[derive(Default)]
pub struct CameraController {
    camera: Camera,
    movement_this_frame: Vec3,
    yaw: f32,
    pitch: f32,
}

const MOUSE_SENSITIVITY: f32 = 0.1;

impl CameraController {
    pub fn input(&mut self, input: DeviceEvent, delta_time: f32) {
        let camera_speed = 5. * delta_time;
        let camera_front = &self.camera.camera_front;
        match input {
            DeviceEvent::MouseMotion { delta: (x, y) } => {
                if x != 0.0 {
                    self.yaw += (x as f32).to_radians() * MOUSE_SENSITIVITY;
                }

                if y != 0.0 {
                    self.pitch -= (y as f32).to_radians() * MOUSE_SENSITIVITY;
                }
            }
            DeviceEvent::Key(keyboard_input) => {
                if keyboard_input.state != ElementState::Pressed {
                    return;
                }

                match keyboard_input.virtual_keycode {
                    Some(VirtualKeyCode::W) => {
                        self.movement_this_frame += camera_front * camera_speed;
                    }
                    Some(VirtualKeyCode::S) => {
                        self.movement_this_frame -= camera_front * camera_speed;
                    }
                    Some(VirtualKeyCode::A) => {
                        self.movement_this_frame -=
                            (camera_front.cross(&Vec3::y())).normalize() * camera_speed;
                    }
                    Some(VirtualKeyCode::D) => {
                        self.movement_this_frame +=
                            (camera_front.cross(&Vec3::y())).normalize() * camera_speed;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn view(&mut self) -> Mat4x4 {
        let camera = &mut self.camera;
        camera.rotate(self.pitch, self.yaw);
        camera.position += self.movement_this_frame;

        self.reset();
        self.camera.to_matrix()
    }

    fn reset(&mut self) {
        self.movement_this_frame = Vec3::zeros();
        self.yaw = 0.;
        self.pitch = 0.;
    }

    pub fn position(&self) -> Vec4 {
        nalgebra_glm::vec3_to_vec4(&self.camera.position)
    }
}
