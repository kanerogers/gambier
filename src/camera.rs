use nalgebra_glm::{self as glm, Mat4x4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub camera_front: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn to_matrix(&self) -> Mat4x4 {
        glm::look_at_rh(
            &self.position,
            &(self.position + &self.camera_front),
            &Vec3::y(),
        )
    }
}

impl Camera {
    pub fn rotate(&mut self, pitch: f32, yaw: f32) {
        self.pitch += pitch;
        self.yaw += yaw;
        self.camera_front = get_camera_front(self.pitch, self.yaw);
    }
}

impl Default for Camera {
    fn default() -> Self {
        let yaw = -std::f32::consts::FRAC_PI_2;
        let pitch = 0.;
        let camera_front = get_camera_front(0., yaw);

        Self {
            position: Vec3::z() * 2.,
            camera_front,
            pitch,
            yaw,
        }
    }
}

fn get_camera_front(pitch: f32, yaw: f32) -> Vec3 {
    Vec3::new(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
    .normalize()
}
