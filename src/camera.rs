// Camera for wgpu
// Left-handed coordinate system

use nalgebra::{Matrix4, UnitQuaternion, Vector3};

type Vec3 = Vector3<f32>;

pub struct Camera {
    position: Vec3,
    // Normalized front(direction) vector
    front: Vec3,
    // Normalized right vector
    right: Vec3,
    // Normalized up vector
    up: Vec3,
    // Yaw angle in degrees (along X axis)
    yaw: f32,
    // Pitch angle in degrees (along Y axis)
    pitch: f32,
}

impl Camera {
    pub fn new(position: [f32; 3], front: [f32; 3]) -> Self {
        let front: Vec3 = Vec3::from(front).normalize();
        let right: Vec3 = Vec3::normalize(&Vec3::cross(&Vec3::y_axis(), &front));
        let up: Vec3 = Vec3::normalize(&Vec3::cross(&front, &right));

        let yaw = front.x.atan2(front.z).to_degrees();
        let pitch = front.y.asin().to_degrees();

        Self {
            position: position.into(),
            front,
            right,
            up,
            yaw,
            pitch,
        }
    }

    pub fn calculate_view_matrix(&mut self) -> Matrix4<f32> {
        // Calculate vectors
        let rot_around_y =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        let rot_around_x =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());
        let total_rot = rot_around_y * rot_around_x;

        self.front = Vec3::normalize(&total_rot.transform_vector(&Vector3::z_axis()));
        self.right = Vec3::normalize(&Vec3::cross(&Vec3::y_axis(), &self.front));
        self.up = Vec3::normalize(&Vec3::cross(&self.front, &self.right));

        let target = self.position + self.front;
        Matrix4::look_at_lh(&self.position.into(), &target.into(), &self.up)
    }

    pub fn set_position(&mut self, new_position: [f32; 3]) {
        self.position = new_position.into();
    }
    pub fn position(&self) -> [f32; 3] {
        self.position.into()
    }
    pub fn add_position(&mut self, add: [f32; 3]) {
        self.position += Vec3::from(add);
    }

    pub fn set_yaw(&mut self, new_yaw: f32) {
        self.yaw = new_yaw % 360.0;
    }
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn set_pitch(&mut self, new_pitch: f32) {
        self.pitch = new_pitch.clamp(-89.0, 89.0);
    }
    pub fn pitch(&self) -> f32 {
        self.pitch
    }
}
