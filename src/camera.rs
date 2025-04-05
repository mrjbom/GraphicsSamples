// Camera for wgpu
// Left-handed coordinate system

use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use std::time::Duration;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::monitor::MonitorHandle;

// For sensitivity correction
// Tested on my screen
// Adjusted for a different screen
const STANDARD_SCREEN_SIZE: (u32, u32) = (2560, 1440);
const STANDARD_SCREEN_SIZE_COEFFICIENT: f32 = 0.15;

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
    // Settings
    sensitivity: f32,
    move_speed: f32,
    // Screen size coefficient for sensitivity correction
    screen_size_coefficient: f32,
    // Input
    move_forward: bool,
    move_back: bool,
    move_right: bool,
    move_left: bool,
    lmb_is_pressed: bool,
}

impl Camera {
    pub fn new(
        position: [f32; 3],
        front: [f32; 3],
        sensitivity: f32,
        move_speed: f32,
        current_monitor: Option<MonitorHandle>,
    ) -> Self {
        let screen_size_coefficient = if let Some(current_monitor) = current_monitor {
            // Corrects standard screen size coefficient to current monitor
            // Example
            // Standard monitor: 2560x1440
            // Standard screen size coefficient: 0.5
            // 1.
            // Current monitor: 1920x1080
            // Difference of standard: 0.75
            // Screen coefficient: 0.75 * 0.5 = 0.375
            // 2.
            // Current monitor: 1280x1024
            // Difference of standard: 0.5
            // Screen coefficient: 0.5 * 0.5 = 0.25
            // 3.
            // Current monitor: 3840x1600
            // Difference of standard: 1.11
            // Difference of standard: 1.11 * 0.5 = 0.555

            // Calculate monitor size coefficient
            let difference_x = current_monitor.size().width as f32 / STANDARD_SCREEN_SIZE.0 as f32;
            let difference_y = current_monitor.size().height as f32 / STANDARD_SCREEN_SIZE.1 as f32;

            let scale_factor = f32::min(difference_x, difference_y);

            STANDARD_SCREEN_SIZE_COEFFICIENT * scale_factor
        } else {
            STANDARD_SCREEN_SIZE_COEFFICIENT
        };

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
            sensitivity,
            move_speed,
            screen_size_coefficient,
            move_forward: false,
            move_back: false,
            move_right: false,
            move_left: false,
            lmb_is_pressed: false,
        }
    }

    pub fn calculate_view_matrix(&mut self, frame_time_delta: Duration) -> Matrix4<f32> {
        // Calculate vectors
        let rot_around_y =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw.to_radians());
        let rot_around_x =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch.to_radians());
        let total_rot = rot_around_y * rot_around_x;

        self.front = Vec3::normalize(&total_rot.transform_vector(&Vector3::z_axis()));
        self.right = Vec3::normalize(&Vec3::cross(&Vec3::y_axis(), &self.front));
        self.up = Vec3::normalize(&Vec3::cross(&self.front, &self.right));

        // Move
        if self.move_forward {
            self.position += self.front * self.move_speed * frame_time_delta.as_secs_f32();
        }
        if self.move_back {
            self.position -= self.front * self.move_speed * frame_time_delta.as_secs_f32();
        }
        if self.move_right {
            self.position += self.right * self.move_speed * frame_time_delta.as_secs_f32();
        }
        if self.move_left {
            self.position -= self.right * self.move_speed * frame_time_delta.as_secs_f32();
        }

        let target = self.position + self.front;
        Matrix4::look_at_lh(&self.position.into(), &target.into(), &self.up)
    }

    pub fn position(&self) -> [f32; 3] {
        self.position.into()
    }
    pub fn set_position(&mut self, new_position: [f32; 3]) {
        self.position = new_position.into();
    }
    pub fn add_position(&mut self, add: [f32; 3]) {
        self.position += Vec3::from(add);
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }
    pub fn set_yaw(&mut self, new_yaw: f32) {
        self.yaw = new_yaw % 360.0;
    }
    pub fn add_yaw(&mut self, add: f32) {
        self.set_yaw(self.yaw + add);
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }
    pub fn set_pitch(&mut self, new_pitch: f32) {
        self.pitch = new_pitch.clamp(-89.0, 89.0);
    }
    pub fn add_pitch(&mut self, add: f32) {
        self.set_pitch(self.pitch + add);
    }

    pub(crate) fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) {
        if key == PhysicalKey::Code(KeyCode::KeyW) {
            self.move_forward = state.is_pressed()
        }
        if key == PhysicalKey::Code(KeyCode::KeyS) {
            self.move_back = state.is_pressed();
        }
        if key == PhysicalKey::Code(KeyCode::KeyD) {
            self.move_right = state.is_pressed();
        }
        if key == PhysicalKey::Code(KeyCode::KeyA) {
            self.move_left = state.is_pressed();
        }
    }

    pub(crate) fn process_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        if self.lmb_is_pressed {
            self.add_yaw(delta_x as f32 * self.sensitivity * self.screen_size_coefficient);
            self.add_pitch(delta_y as f32 * self.sensitivity * self.screen_size_coefficient);
        }
    }

    pub(crate) fn process_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left {
            self.lmb_is_pressed = state.is_pressed();
        }
    }
}
