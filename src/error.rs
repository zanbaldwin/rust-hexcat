use error_stack::{Context};
use std::fmt::Display;

#[derive(Debug)]
pub enum InitError {
    NotEnoughArguments,
    InvalidConnectionSettings,
    CouldNotConnect,
    NoTerminal,
}
impl Context for InitError {}
impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("App could not start.")
    }
}

#[derive(Debug)]
pub enum AppError {
    InitError,
    CouldNotPaint,
    TerminalError,
    UserInput,
    StreamRead,
    StreamWrite,
}
impl Context for AppError {}
impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Something went wrong.")
    }
}
