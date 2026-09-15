use super::*;

const CLEAR: Color = Color::rgba(0, 0, 0, 0);
const INK: Color = Color::rgb(221, 47, 93);

#[test]
fn circular_coverage_has_a_solid_interior_and_symmetric_partial_edges() {
    let mut pixels = PixelBuffer::new(13, 13, CLEAR).unwrap();
    pixels.stamp_circle(6.5, 6.5, 4.5, INK);
    assert_eq!(pixels.get_pixel(6, 6), Some(INK));
    assert_eq!(pixels.get_pixel(9, 6), Some(INK));
    assert!((1..255).contains(&pixels.get_pixel(9, 9).unwrap().alpha()));
    assert_eq!(pixels.get_pixel(10, 10), Some(CLEAR));
    for y in 0..13 {
        for x in 0..13 {
            assert_eq!(pixels.get_pixel(x, y), pixels.get_pixel(12 - x, y));
            assert_eq!(pixels.get_pixel(x, y), pixels.get_pixel(y, x));
        }
    }
}

#[test]
fn fractional_centers_and_radii_change_coverage_gradually() {
    let mut previous_left = 256;
    let mut previous_right = 0;
    for step in 0..=16 {
        let mut pixels = PixelBuffer::new(4, 3, CLEAR).unwrap();
        pixels.stamp_circle(1.5 + step as f32 / 16.0, 1.5, 0.5, INK);
        let left = u16::from(pixels.get_pixel(1, 1).unwrap().alpha());
        let right = u16::from(pixels.get_pixel(2, 1).unwrap().alpha());
        assert!(left < previous_left);
        assert!(right >= previous_right && right - previous_right <= 16);
        assert!((254..=256).contains(&(left + right)));
        previous_left = left;
        previous_right = right;
    }
    let mut previous = 0;
    for step in 0..=16 {
        let mut pixels = PixelBuffer::new(5, 5, CLEAR).unwrap();
        pixels.stamp_circle(2.5, 2.5, 0.5 + step as f32 / 16.0, INK);
        let alpha = u16::from(pixels.get_pixel(3, 2).unwrap().alpha());
        assert!(alpha >= previous && alpha - previous <= 16);
        previous = alpha;
    }
    assert_eq!(previous, 255);
}

#[test]
fn small_circles_remain_visible_and_approximate_geometric_area() {
    // Independent geometric reference: integrate membership in the circle on
    // a 64 x 64 subpixel grid, not the rasterizer's distance ramp. That ramp is
    // approximate: allow 1/4 pixel of local error and 0.6 pixel of total area.
    for radius in [0.5_f32, 0.75, 1.0, 1.25, 2.5, 5.25] {
        for offset in [0.0, 0.25, 0.5, 0.75] {
            let cx = 7.0 + offset;
            let cy = 7.25;
            let mut pixels = PixelBuffer::new(15, 15, CLEAR).unwrap();
            pixels.stamp_circle(cx, cy, radius, INK);
            let mut area = 0.0;
            for y in 0..15 {
                for x in 0..15 {
                    let coverage = f64::from(pixels.get_pixel(x, y).unwrap().alpha()) / 255.0;
                    area += coverage;
                    let mut inside = 0;
                    for sy in 0..64 {
                        for sx in 0..64 {
                            let dx = f64::from(x) + (f64::from(sx) + 0.5) / 64.0 - f64::from(cx);
                            let dy = f64::from(y) + (f64::from(sy) + 0.5) / 64.0 - f64::from(cy);
                            inside += u32::from(dx * dx + dy * dy <= f64::from(radius).powi(2));
                        }
                    }
                    let reference = f64::from(inside) / 4096.0;
                    assert!(
                        (coverage - reference).abs() < 0.25,
                        "r={radius}, center=({cx},{cy}), pixel=({x},{y}): {coverage} vs {reference}"
                    );
                }
            }
            let reference_area = std::f64::consts::PI * f64::from(radius).powi(2);
            assert!(
                (area - reference_area).abs() < 0.6,
                "r={radius}: {area} vs {reference_area}"
            );
            assert!(area >= 0.9, "the minimum brush must stay visible");
        }
    }
}

#[test]
fn paint_and_erase_modulate_alpha_without_color_halos() {
    let background = Color::rgba(20, 80, 190, 180);
    let hidden = Color::rgba(0, 255, 0, 0);
    let white = Color::rgb(255, 255, 255);
    let source = Color::rgba(INK.red(), INK.green(), INK.blue(), 128);
    let mut mask = PixelBuffer::new(10, 10, hidden).unwrap();
    let mut painted = PixelBuffer::new(10, 10, white).unwrap();
    let mut erased = PixelBuffer::new(10, 10, background).unwrap();
    mask.stamp_circle(4.75, 5.25, 3.5, source);
    painted.stamp_circle(4.75, 5.25, 3.5, source);
    erased.erase_circle(4.75, 5.25, 3.5, 128);
    let mut partial = 0;
    for index in 0..mask.pixels.len() {
        let ink = Color::from_u32(mask.pixels[index]);
        let remaining = Color::from_u32(erased.pixels[index]);
        assert_eq!(
            Color::from_u32(painted.pixels[index]),
            ink.blend_over(white)
        );
        assert_eq!(
            (remaining.red(), remaining.green(), remaining.blue()),
            (20, 80, 190)
        );
        assert_eq!(
            remaining.alpha(),
            ((180_u16 * (255 - u16::from(ink.alpha())) + 127) / 255) as u8
        );
        if ink.alpha() > 0 {
            assert_eq!(
                (ink.red(), ink.green(), ink.blue()),
                (INK.red(), INK.green(), INK.blue())
            );
        }
        partial += usize::from(ink.alpha() > 0 && ink.alpha() < 128);
    }
    assert!(partial > 8);
    erased.erase_circle(4.75, 5.25, 3.5, 255);
    assert_eq!(erased.get_pixel(4, 5).unwrap().alpha(), 0);
    mask.erase_circle(4.75, 5.25, 3.5, 255);
    assert_eq!(mask.get_pixel(0, 0), Some(hidden));
}

#[test]
fn clipping_matches_cropping_on_all_sides_and_tiny_documents() {
    for (width, height) in [(1, 1), (1, 7), (7, 1), (5, 4)] {
        for (cx, cy) in [
            (-0.25, -0.75),
            (0.5, height as f32),
            (width as f32, 0.25),
            (width as f32 + 0.5, height as f32 + 0.25),
            (2.25, 1.75),
            (-15.0, 0.0),
        ] {
            for radius in [0.5, 2.75, 12.25] {
                for erase in [false, true] {
                    let fill = if erase { INK } else { CLEAR };
                    let mut small = PixelBuffer::new(width, height, fill).unwrap();
                    let mut large = PixelBuffer::new(width + 64, height + 64, fill).unwrap();
                    if erase {
                        small.erase_circle(cx, cy, radius, 190);
                        large.erase_circle(cx + 32.0, cy + 32.0, radius, 190);
                    } else {
                        small.stamp_circle(cx, cy, radius, INK);
                        large.stamp_circle(cx + 32.0, cy + 32.0, radius, INK);
                    }
                    for y in 0..height {
                        for x in 0..width {
                            assert_eq!(small.get_pixel(x, y), large.get_pixel(x + 32, y + 32));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn invalid_or_transparent_dabs_leave_pixels_intact() {
    let mut pixels = PixelBuffer::new(3, 3, INK).unwrap();
    let original = pixels.pixels.clone();
    for (x, y, radius) in [
        (f32::NAN, 1.0, 2.0),
        (1.0, f32::INFINITY, 2.0),
        (1.0, 1.0, f32::INFINITY),
        (1.0, 1.0, -1.0),
        (1.0, 1.0, 0.0),
    ] {
        pixels.stamp_circle(x, y, radius, Color::rgb(0, 0, 0));
        pixels.erase_circle(x, y, radius, 255);
    }
    pixels.stamp_circle(1.0, 1.0, 2.0, CLEAR);
    pixels.erase_circle(1.0, 1.0, 2.0, 0);
    assert_eq!(pixels.pixels, original);
}
