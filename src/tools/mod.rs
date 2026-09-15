mod brush;
mod dynamics;
mod eraser;
mod stroke;

pub use brush::BrushTool;
pub use eraser::EraserTool;
pub use stroke::BrushSample;

use crate::document::PixelBuffer;
use dynamics::BrushDynamics;
use stroke::Stroke;

pub trait Tool {
    fn pointer_down(&mut self, pixels: &mut PixelBuffer, sample: BrushSample);
    fn pointer_move(&mut self, pixels: &mut PixelBuffer, sample: BrushSample);
    fn pointer_up(&mut self, pixels: &mut PixelBuffer);
    fn cancel(&mut self);
    fn break_segment(&mut self, pixels: &mut PixelBuffer);
    fn is_active(&self) -> bool;
}
