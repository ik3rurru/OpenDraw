/// A normalized input sample after conversion to document coordinates.
/// The sampler and tools have no dependency on a native input API.
/// The document starts at (0, 0); pixel (x, y) is centered at (x + 0.5, y + 0.5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSample {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
    pub timestamp: u64,
}

impl BrushSample {
    /// Discrete cell for the bucket/eyedropper; brush dabs retain x/y as floats.
    pub fn pixel(self) -> (u32, u32) {
        (self.x as u32, self.y as u32)
    }

    fn interpolate(self, other: Self, t: f64) -> Self {
        let lerp = |a: f32, b: f32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t) as f32;
        let rotation_delta = (other.rotation - self.rotation + 180.0).rem_euclid(360.0) - 180.0;
        Self {
            x: lerp(self.x, other.x),
            y: lerp(self.y, other.y),
            pressure: lerp(self.pressure, other.pressure),
            tilt_x: lerp(self.tilt_x, other.tilt_x),
            tilt_y: lerp(self.tilt_y, other.tilt_y),
            rotation: lerp(self.rotation, self.rotation + rotation_delta).rem_euclid(360.0),
            timestamp: if other.timestamp >= self.timestamp {
                self.timestamp
                    .saturating_add(((other.timestamp - self.timestamp) as f64 * t) as u64)
            } else {
                self.timestamp
                    .saturating_sub(((self.timestamp - other.timestamp) as f64 * t) as u64)
            },
        }
    }
}

#[derive(Default)]
pub(super) struct Stroke {
    pub active: bool,
    previous_sample: Option<BrushSample>,
    last_stamp: Option<BrushSample>,
    // Accumulate distance at higher precision so splitting a segment into many
    // input packets does not move its dabs or stamp the endpoint twice.
    distance_to_next: f64,
}

// Half-pixel spacing keeps one-pixel diagonals continuous without a dab for
// every device event. Larger brushes retain quarter-diameter spacing.
const MIN_SPACING: f32 = 0.5;
const POSITION_EPSILON: f32 = 0.0001;

impl Stroke {
    pub fn pointer_down(
        &mut self,
        sample: BrushSample,
        spacing: impl Fn(BrushSample) -> f32,
        mut stamp: impl FnMut(BrushSample),
    ) {
        if !sample.x.is_finite() || !sample.y.is_finite() {
            return;
        }
        self.active = true;
        self.previous_sample = Some(sample);
        self.last_stamp = None;
        self.distance_to_next = f64::from(spacing(sample).max(MIN_SPACING));
        self.stamp(sample, &mut stamp);
    }

    pub fn pointer_move(
        &mut self,
        sample: BrushSample,
        spacing: impl Fn(BrushSample) -> f32,
        mut stamp: impl FnMut(BrushSample),
    ) {
        if !self.active || !sample.x.is_finite() || !sample.y.is_finite() {
            return;
        }
        let Some(previous) = self.previous_sample else {
            self.pointer_down(sample, spacing, stamp);
            return;
        };
        let distance = (f64::from(sample.x) - f64::from(previous.x))
            .hypot(f64::from(sample.y) - f64::from(previous.y));
        if distance > 0.0 {
            // Carry the unused distance between messages. Stamping every event's
            // endpoint would make opacity depend on the tablet's sample rate.
            let mut traveled = self.distance_to_next;
            while traveled <= distance {
                let next = previous.interpolate(sample, traveled / distance);
                self.stamp(next, &mut stamp);
                traveled += f64::from(spacing(next).max(MIN_SPACING));
            }
            self.distance_to_next = traveled - distance;
        }
        self.previous_sample = Some(sample);
    }

    pub fn pointer_up(&mut self, stamp: impl FnMut(BrushSample)) {
        self.break_segment(stamp);
        self.active = false;
    }

    pub fn cancel(&mut self) {
        *self = Self::default();
    }

    pub fn break_segment(&mut self, mut stamp: impl FnMut(BrushSample)) {
        if let Some(sample) = self.previous_sample {
            self.stamp(sample, &mut stamp);
        }
        self.previous_sample = None;
        self.last_stamp = None;
        self.distance_to_next = 0.0;
    }

    fn stamp(&mut self, sample: BrushSample, stamp: &mut impl FnMut(BrushSample)) {
        // Keep distinct fractional centers, including the last contact. Only
        // discard stationary packets and roundoff at the segment endpoint.
        // Pressure-only changes take effect at the next stamp position.
        if self.last_stamp.is_none_or(|last| {
            (last.x - sample.x).abs() > POSITION_EPSILON
                || (last.y - sample.y).abs() > POSITION_EPSILON
        }) {
            stamp(sample);
            self.last_stamp = Some(sample);
        }
    }
}

#[cfg(test)]
pub(super) fn sample(x: f32, y: f32, pressure: f32) -> BrushSample {
    BrushSample {
        x,
        y,
        pressure,
        tilt_x: 0.0,
        tilt_y: 0.0,
        rotation: 0.0,
        timestamp: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_position_pressure_tilt_rotation_and_time() {
        let mut a = sample(1.25, 2.5, 0.2);
        a.rotation = 359.0;
        a.timestamp = 10;
        let mut b = sample(11.25, 22.5, 1.0);
        b.tilt_x = -40.0;
        b.tilt_y = 20.0;
        b.rotation = 1.0;
        b.timestamp = 30;
        let middle = a.interpolate(b, 0.5);
        assert_eq!((middle.x, middle.y), (6.25, 12.5));
        assert!((middle.pressure - 0.6).abs() < 0.0001);
        assert_eq!(
            (middle.tilt_x, middle.tilt_y, middle.rotation),
            (-20.0, 10.0, 0.0)
        );
        assert_eq!(middle.timestamp, 20);
    }

    #[test]
    fn spacing_survives_small_packets_and_finishes_at_the_last_contact() {
        fn draw(step: f32) -> Vec<(u32, u32)> {
            let mut stroke = Stroke::default();
            let mut points = Vec::new();
            let mut stamp = |sample: BrushSample| points.push(sample.pixel());
            stroke.pointer_down(sample(0.0, 0.0, 1.0), |_| 3.0, &mut stamp);
            for i in 1..=(10.0 / step) as u32 {
                stroke.pointer_move(sample(i as f32 * step, 0.0, 1.0), |_| 3.0, &mut stamp);
            }
            stroke.pointer_up(&mut stamp);
            points
        }
        assert_eq!(draw(10.0), [(0, 0), (3, 0), (6, 0), (9, 0), (10, 0)]);
        assert_eq!(draw(0.25), draw(10.0));
    }

    #[test]
    fn duplicate_samples_and_cancellation_do_not_add_dabs() {
        let mut stroke = Stroke::default();
        let mut points = Vec::new();
        let mut stamp = |sample: BrushSample| points.push(sample.pixel());
        let a = sample(4.5, 5.5, 0.5);
        stroke.pointer_down(a, |_| 3.0, &mut stamp);
        for _ in 0..100 {
            stroke.pointer_move(a, |_| 3.0, &mut stamp);
        }
        stroke.pointer_up(&mut stamp);
        stroke.pointer_move(sample(20.0, 5.0, 1.0), |_| 3.0, &mut stamp);
        assert_eq!(points, [(4, 5)]);
        stroke.cancel();
        assert!(!stroke.active);
    }

    #[test]
    fn fractional_dabs_share_a_pixel_and_flush_the_last_contact_only_once() {
        let mut stroke = Stroke::default();
        let mut points = Vec::new();
        let mut stamp = |sample: BrushSample| points.push((sample.x, sample.y));
        stroke.pointer_down(sample(4.1, 5.5, 0.5), |_| 0.5, &mut stamp);
        stroke.pointer_move(sample(4.85, 5.5, 0.5), |_| 0.5, &mut stamp);
        stroke.pointer_up(&mut stamp);
        stroke.pointer_up(&mut stamp);
        assert_eq!(points.len(), 3);
        for (point, expected) in points.iter().zip([4.1, 4.6, 4.85]) {
            assert!((point.0 - expected).abs() < 0.00001);
            assert_eq!(point.1, 5.5);
        }
    }

    #[test]
    fn pressure_only_packets_and_endpoint_roundoff_do_not_repeat_a_dab() {
        let mut stroke = Stroke::default();
        let mut points = Vec::new();
        let mut stamp = |sample: BrushSample| points.push(sample);
        stroke.pointer_down(sample(4.25, 5.5, 0.5), |_| 0.5, &mut stamp);
        for i in 0..100 {
            stroke.pointer_move(sample(4.25, 5.5, i as f32 / 100.0), |_| 0.5, &mut stamp);
        }
        stroke.pointer_move(sample(4.750001, 5.5, 0.5), |_| 0.5, &mut stamp);
        stroke.pointer_up(&mut stamp);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn invalid_positions_do_not_break_the_sampler() {
        let mut stroke = Stroke::default();
        let mut points = Vec::new();
        let mut stamp = |sample: BrushSample| points.push(sample.x);
        stroke.pointer_down(sample(f32::NAN, 0.0, 1.0), |_| 0.5, &mut stamp);
        assert!(!stroke.active);
        stroke.pointer_down(sample(1.0, 0.0, 1.0), |_| 0.5, &mut stamp);
        stroke.pointer_move(sample(f32::INFINITY, 0.0, 1.0), |_| 0.5, &mut stamp);
        stroke.pointer_move(sample(2.0, 0.0, 1.0), |_| 0.5, &mut stamp);
        stroke.pointer_up(&mut stamp);
        assert_eq!(points, [1.0, 1.5, 2.0]);
    }
}
