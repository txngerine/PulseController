use crate::controller::{StickMapping, StickCurve};

#[derive(Debug, Clone)]
pub struct MouseMapper {
    sensitivity: f32,
    acceleration: bool,
    scroll_sensitivity: f32,
}

impl MouseMapper {
    pub fn new() -> Self {
        Self {
            sensitivity: 1.0,
            acceleration: false,
            scroll_sensitivity: 1.0,
        }
    }

    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity;
    }

    pub fn set_acceleration(&mut self, enabled: bool) {
        self.acceleration = enabled;
    }

    pub fn set_scroll_sensitivity(&mut self, sensitivity: f32) {
        self.scroll_sensitivity = sensitivity;
    }

    pub fn process_stick_movement(
        &self,
        x: f32,
        y: f32,
        mapping: &StickMapping,
    ) -> (i32, i32) {
        let (processed_x, processed_y) = apply_curve(x, y, &mapping.curve);

        let dx = (processed_x * mapping.sensitivity * self.sensitivity * 100.0) as i32;
        let dy = (processed_y * mapping.sensitivity * self.sensitivity * 100.0) as i32;

        if self.acceleration {
            let speed = ((dx * dx + dy * dy) as f32).sqrt();
            let accel = 1.0 + speed * 0.001;
            ((dx as f32 * accel) as i32, (dy as f32 * accel) as i32)
        } else {
            (dx, dy)
        }
    }

    pub fn process_scroll(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x * self.scroll_sensitivity) as i32,
            (y * self.scroll_sensitivity) as i32,
        )
    }
}

fn apply_curve(x: f32, y: f32, curve: &StickCurve) -> (f32, f32) {
    match curve {
        StickCurve::Linear => (x, y),
        StickCurve::Exponential => {
            let magnitude = (x * x + y * y).sqrt();
            if magnitude < 0.001 {
                return (0.0, 0.0);
            }
            let normalized = magnitude.min(1.0);
            let curved = normalized.powf(2.0);
            let scale = curved / magnitude;
            (x * scale, y * scale)
        }
        StickCurve::Aggressive => {
            let magnitude = (x * x + y * y).sqrt();
            if magnitude < 0.001 {
                return (0.0, 0.0);
            }
            let normalized = magnitude.min(1.0);
            let curved = if normalized < 0.5 {
                normalized * 0.5
            } else {
                0.25 + (normalized - 0.5) * 1.5
            };
            let scale = curved / magnitude;
            (x * scale, y * scale)
        }
    }
}

impl Default for MouseMapper {
    fn default() -> Self {
        Self::new()
    }
}
