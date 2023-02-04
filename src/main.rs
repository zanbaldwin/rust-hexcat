mod error;
mod paint;
mod sections;
mod terminal;
mod window;

use crate::error::{AppError, InitError};
use crate::window::{Window, WindowReceiver};
use error_stack::{IntoReport, Result, ResultExt};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;
use std::{env, thread};
use terminal::Terminal;
use termion::event::Key;

type TcpMessage = Vec<u8>;

const BUFFER_SIZE: usize = 4_096;

pub(crate) enum MessageOrigin {
    Local(TcpMessage),
    Remote(TcpMessage),
}

// Don't hog an entire CPU core at 100% in the infinite loop. Chill out for a little bit each iteration.
pub(crate) const THREAD_SLOW_DOWN: Duration = Duration::from_millis(5);

fn main() -> Result<ExitCode, AppError> {
    let mut window: Window = start_window()
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
    let thread_connection = connection
        .try_clone()
        .into_report()
        .attach_printable("Could not clone connection for use in TCP thread.")
        .change_context(InitError::Threads)?;

    let window = Window::new(terminal, connection, spawn_threads(thread_connection))
        .attach_printable("Could not initialize terminal window.")
        .change_context(InitError::Window)?;

    Ok(window)
}

fn connect() -> Result<TcpStream, InitError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        Err(InitError::NotEnoughArguments)
            .into_report()
            .attach_printable("You must supply at least 2 arguments (IP Address and Port).")?;
    }

    let addr: IpAddr = args[1]
        .parse()
        .into_report()
        .attach_printable("Invalid IP address.")
        .change_context(InitError::InvalidConnectionSettings)?;
    let port: u16 = args[2]
        .parse()
        .into_report()
        .attach_printable("Invalid port number.")
        .change_context(InitError::InvalidConnectionSettings)?;

    let socket_addr: SocketAddr = SocketAddr::new(addr, port);
    let stream = TcpStream::connect(socket_addr)
        .into_report()
        .attach_printable(format!(
            "Could not connect to remote server (using {addr} on port {port})."
        ))
        .change_context(InitError::CouldNotConnect)?;

    Ok(stream)
}

fn spawn_threads(connection: TcpStream) -> WindowReceiver {
    let (message_sink, message_receiver) = mpsc::channel::<TcpMessage>();
    thread::spawn(move || sections::Messages::listen(connection, message_sink));
    let (input_sink, input_receiver) = mpsc::channel::<Key>();
    thread::spawn(move || sections::Input::listen(input_sink));

    WindowReceiver::new(message_receiver, input_receiver)
}
