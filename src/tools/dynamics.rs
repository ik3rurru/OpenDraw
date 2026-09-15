/// Pressure response shared by painting and erasing. Amounts and minimum_size
/// are fractions: zero disables a response, one applies it fully.
#[derive(Clone, Copy, Debug)]
pub struct BrushDynamics {
    pub pressure_size: f32,
    pub pressure_opacity: f32,
    pub minimum_size: f32,
}

impl Default for BrushDynamics {
    fn default() -> Self {
        Self {
            pressure_size: 1.0,
            pressure_opacity: 1.0,
            minimum_size: 0.1,
        }
    }
}

impl BrushDynamics {
    /// Geometric radius in document pixels. The size control still denotes an
    /// odd diameter (2 * base_radius + 1), with a one-pixel minimum diameter.
    pub fn radius(self, base_radius: u32, pressure: f32) -> f32 {
        let minimum = self.minimum_size.clamp(0.0, 1.0);
        let response = minimum + unit_pressure(pressure) * (1.0 - minimum);
        let factor = 1.0 + (response - 1.0) * self.pressure_size.clamp(0.0, 1.0);
        let diameter = (base_radius as f32 * 2.0 + 1.0) * factor;
        diameter.max(1.0) / 2.0
    }

    pub fn opacity(self, base_opacity: u8, pressure: f32) -> u8 {
        let factor = 1.0 + (unit_pressure(pressure) - 1.0) * self.pressure_opacity.clamp(0.0, 1.0);
        (f32::from(base_opacity) * factor).round() as u8
    }
}

fn unit_pressure(pressure: f32) -> f32 {
    if pressure.is_finite() {
        pressure.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_changes_size_and_alpha_with_a_minimum_diameter() {
        let dynamics = BrushDynamics::default();
        assert!((dynamics.radius(20, 0.0) - 2.05).abs() < 0.00001);
        assert!((dynamics.radius(20, 0.5) - 11.275).abs() < 0.00001);
        assert_eq!(dynamics.radius(20, 1.0), 20.5);
        assert_eq!(dynamics.radius(0, 0.5), 0.5);
        assert_eq!(dynamics.opacity(200, 0.5), 100);
        assert_eq!(dynamics.opacity(200, -1.0), 0);
        assert_eq!(dynamics.opacity(200, 2.0), 200);
    }

    #[test]
    fn pressure_responses_can_be_disabled_independently() {
        let mut dynamics = BrushDynamics {
            pressure_size: 0.0,
            ..BrushDynamics::default()
        };
        assert_eq!(dynamics.radius(20, 0.2), 20.5);
        assert_eq!(dynamics.opacity(200, 0.2), 40);
        dynamics.pressure_opacity = 0.0;
        assert_eq!(dynamics.opacity(200, 0.2), 200);
    }
}
