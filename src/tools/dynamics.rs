/// Response to the physical pressure of the pen: a quadratic Bezier from
/// (0, 0) to (1, 1) bent towards one control point. Above the diagonal a light
/// touch already responds strongly (soft); below it the pen needs more force
/// (firm). Every control point on the diagonal is linear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureCurve {
    x: f32,
    y: f32,
}

impl Default for PressureCurve {
    fn default() -> Self {
        Self::LINEAR
    }
}

impl PressureCurve {
    pub const LINEAR: Self = Self { x: 0.5, y: 0.5 };

    /// Coordinates are clamped to 0..=1; a non-finite one gives a linear curve.
    pub fn new(x: f32, y: f32) -> Self {
        if !x.is_finite() || !y.is_finite() {
            return Self::LINEAR;
        }
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
        }
    }

    pub fn control(self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Maps a raw pressure to the response seen by the tools. Both ends are
    /// exact, so a mouse at full pressure is unaffected by any curve.
    pub fn apply(self, pressure: f32) -> f32 {
        let pressure = unit_pressure(pressure);
        if self.x == self.y || pressure == 0.0 || pressure == 1.0 {
            return pressure;
        }
        let (cx, cy) = (f64::from(self.x), f64::from(self.y));
        let x = f64::from(pressure);
        // Solve x = 2t(1 - t) * cx + t^2 for t. The rationalized root avoids
        // cancellation near cx = 0.5. Its discriminant, (1 - x)cx^2 + x(1 - cx)^2,
        // is positive for 0 < x < 1, so the denominator never reaches zero.
        let t = x / (cx + (cx * cx + (1.0 - 2.0 * cx) * x).sqrt());
        let y = 2.0 * t * (1.0 - t) * cy + t * t;
        (y as f32).clamp(0.0, 1.0)
    }
}

/// Pressure response shared by painting and erasing. Amounts and minimum_size
/// are fractions: zero disables a response, one applies it fully. The pressure
/// passes through `curve` once per dab, after the stroke is interpolated.
#[derive(Clone, Copy, Debug)]
pub struct BrushDynamics {
    pub pressure_size: f32,
    pub pressure_opacity: f32,
    pub minimum_size: f32,
    pub curve: PressureCurve,
}

impl Default for BrushDynamics {
    fn default() -> Self {
        Self {
            pressure_size: 1.0,
            pressure_opacity: 1.0,
            minimum_size: 0.1,
            curve: PressureCurve::LINEAR,
        }
    }
}

impl BrushDynamics {
    /// Geometric radius in document pixels. The size control still denotes an
    /// odd diameter (2 * base_radius + 1), with a one-pixel minimum diameter.
    pub fn radius(self, base_radius: u32, pressure: f32) -> f32 {
        let minimum = self.minimum_size.clamp(0.0, 1.0);
        let response = minimum + self.curve.apply(pressure) * (1.0 - minimum);
        let factor = 1.0 + (response - 1.0) * self.pressure_size.clamp(0.0, 1.0);
        let diameter = (base_radius as f32 * 2.0 + 1.0) * factor;
        diameter.max(1.0) / 2.0
    }

    pub fn opacity(self, base_opacity: u8, pressure: f32) -> u8 {
        let factor =
            1.0 + (self.curve.apply(pressure) - 1.0) * self.pressure_opacity.clamp(0.0, 1.0);
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

    #[test]
    fn pressure_curve_ends_are_exact_and_the_diagonal_is_linear() {
        for (x, y) in [(0.5, 0.5), (0.0, 1.0), (1.0, 0.0), (0.2, 0.9), (0.9, 0.2)] {
            let curve = PressureCurve::new(x, y);
            assert_eq!(curve.apply(0.0), 0.0);
            assert_eq!(curve.apply(1.0), 1.0);
            assert_eq!(curve.apply(-1.0), 0.0);
            assert_eq!(curve.apply(2.0), 1.0);
            assert_eq!(curve.apply(f32::NAN), 0.0);
        }
        for control in [0.0, 0.3, 0.5, 1.0] {
            let curve = PressureCurve::new(control, control);
            for step in 0..=100 {
                let pressure = step as f32 / 100.0;
                assert_eq!(curve.apply(pressure), pressure);
            }
        }
        assert_eq!(PressureCurve::new(-1.0, 3.0).control(), (0.0, 1.0));
        assert_eq!(PressureCurve::new(f32::NAN, 0.2), PressureCurve::LINEAR);
    }

    #[test]
    fn pressure_curves_bend_monotonically_and_mirror_into_their_inverse() {
        // Control (0, 1) gives y = 2 sqrt(x) - x; (0.5, 1) gives y = 2x - x^2.
        assert!((PressureCurve::new(0.0, 1.0).apply(0.25) - 0.75).abs() < 0.00001);
        assert!((PressureCurve::new(1.0, 0.0).apply(0.75) - 0.25).abs() < 0.00001);
        assert!((PressureCurve::new(0.5, 1.0).apply(0.5) - 0.75).abs() < 0.00001);
        let soft = PressureCurve::new(0.25, 0.75);
        let firm = PressureCurve::new(0.75, 0.25);
        assert!(soft.apply(0.5) > 0.5);
        assert!(firm.apply(0.5) < 0.5);

        for x in 0..=8 {
            for y in 0..=8 {
                let curve = PressureCurve::new(x as f32 / 8.0, y as f32 / 8.0);
                let mirrored = PressureCurve::new(y as f32 / 8.0, x as f32 / 8.0);
                let mut previous = 0.0;
                for step in 0..=1000 {
                    let pressure = step as f32 / 1000.0;
                    let response = curve.apply(pressure);
                    assert!((0.0..=1.0).contains(&response));
                    assert!(response >= previous, "({x}, {y}) at {pressure}");
                    previous = response;
                    // Swapping the control coordinates reflects the curve
                    // across the diagonal, so the two curves undo each other.
                    // Inverting a steep end amplifies f32 rounding.
                    if (0.05..=0.95).contains(&response) {
                        assert!(
                            (mirrored.apply(response) - pressure).abs() < 0.001,
                            "({x}, {y}) at {pressure}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn pressure_curve_shapes_size_and_opacity_but_not_full_pressure() {
        let dynamics = BrushDynamics {
            curve: PressureCurve::new(0.0, 1.0),
            ..BrushDynamics::default()
        };
        // Pressure 25% responds as 75%: diameter 41 * (0.1 + 0.75 * 0.9).
        assert!((dynamics.radius(20, 0.25) - 15.8875).abs() < 0.0001);
        assert_eq!(dynamics.opacity(200, 0.25), 150);
        assert_eq!(dynamics.radius(20, 1.0), 20.5);
        assert_eq!(dynamics.opacity(200, 1.0), 200);
        assert!((dynamics.radius(20, 0.0) - 2.05).abs() < 0.00001);
        assert_eq!(dynamics.opacity(200, 0.0), 0);
    }
}
