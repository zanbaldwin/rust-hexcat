mod error;
mod terminal;
mod window;

use crate::error::{InitError, AppError};
use crate::window::{Window};
use error_stack::{IntoReport, Result, ResultExt};
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::env;
use std::process::ExitCode;
use terminal::Terminal;

fn main() -> Result<ExitCode, AppError> {
    let mut window = start_window()
        .attach_printable("Could not start application due to initialization errors.")
        .change_context(AppError::InitError)?;
    window.run()?;

    Ok(ExitCode::SUCCESS)
}

fn start_window() -> Result<Window, InitError> {
    let terminal: Terminal = Terminal::init()
        .attach_printable("Could not initialize terminal.")
        .change_context(InitError::NoTerminal)?;
    let connection = connect()?;

    Ok(Window::new(terminal, connection))
}

fn connect() -> Result<TcpStream, InitError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        Err(InitError::NotEnoughArguments)
            .into_report()
            .attach_printable("You must supply at least 2 arguments (IP Address and Port).")?
    }

    let addr : IpAddr = args[1].parse()
        .into_report()
        .attach_printable("Invalid IP address.")
        .change_context(InitError::InvalidConnectionSettings)?;
    let port: u16 = args[2].parse()
        .into_report()
        .attach_printable("Invalid port number.")
        .change_context(InitError::InvalidConnectionSettings)?;

    let socket_addr: SocketAddr = SocketAddr::new(addr, port);
    let stream = TcpStream::connect(socket_addr)
        .into_report()
        .attach_printable(format!("Could not connect to remote server (using {addr} on port {port})."))
        .change_context(InitError::CouldNotConnect)?;

    Ok(stream)
}
