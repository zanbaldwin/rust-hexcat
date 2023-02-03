use crate::error::AppError;
use crate::terminal::{Position, Size};
use error_stack::Result;

pub type PaintOutput = Vec<Vec<char>>;

pub trait Painter {
    fn paint(&self, bounds: Size) -> Result<PaintOutput, AppError>;
}

pub struct Paintable<'a, S, P>
where
    S: Fn(Size) -> Size,
    P: Fn(Size) -> Position,
{
    painter: &'a dyn Painter,
    sizer: Box<S>,
    positioner: Box<P>,
}
impl<'a, S, P> Paintable<'a, S, P>
where
    S: Fn(Size) -> Size,
    P: Fn(Size) -> Position,
{
    pub fn size(&self, terminal_size: Size) -> Size {
        (*self.sizer)(terminal_size)
    }
}
impl<'a, S, P> Painter for Paintable<'a, S, P>
where
    S: Fn(Size) -> Size,
    P: Fn(Size) -> Position,
{
    fn paint(&self, bounds: Size) -> Result<PaintOutput, AppError> {
        self.painter.paint(bounds)
    }
}
