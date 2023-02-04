use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InitError {
    NotEnoughArguments,
    InvalidConnectionSettings,
    CouldNotConnect,
    NoTerminal,
    Window,
    Threads,
}
impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("App could not start.")
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    InitError,
    ChannelBroken,
    TerminalError,
    UserInput,
    StreamRead,
}
impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Something went wrong.")
    }
}
