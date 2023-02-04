use crate::error::AppError;
use crate::terminal::Size;
use error_stack::Result;

pub type PaintOutput = Vec<Vec<char>>;

pub trait Painter {
    fn paint(&self, bounds: Size) -> Result<PaintOutput, AppError>;
}
