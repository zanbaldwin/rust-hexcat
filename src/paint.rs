use crate::error::AppError;
use crate::terminal::{Position, Size};
use error_stack::Result;

pub type PaintOutput = Vec<Vec<char>>;

pub trait Painter {
    fn paint(&self, width: usize, height: usize) -> Result<PaintOutput, AppError>;
}
pub struct Paintable<'a> {
    painter: &'a dyn Painter,
    position: Position,
    bounds: Size,
}
impl<'a> Paintable<'a> {
    pub fn bounds(&self) -> Size {
        self.bounds.clone()
    }
}
impl<'a> Painter for Paintable<'a> {
    fn paint(&self, width: usize, height: usize) -> Result<PaintOutput, AppError> {
        self.painter.paint(width, height)
    }
}
