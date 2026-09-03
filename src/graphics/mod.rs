mod color;
mod font;
mod framebuffer;
mod rect;

pub use color::Color;
pub use framebuffer::FrameBuffer;
pub use rect::Rect;

pub(crate) fn rasterize_line(
    mut x0: i64,
    mut y0: i64,
    x1: i64,
    y1: i64,
    mut plot: impl FnMut(i64, i64),
) {
    let dx = (x1 - x0).abs();
    let step_x = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let step_y = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;

    loop {
        plot(x0, y0);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let doubled_error = 2 * error;
        if doubled_error >= dy {
            error += dy;
            x0 += step_x;
        }
        if doubled_error <= dx {
            error += dx;
            y0 += step_y;
        }
    }
}
