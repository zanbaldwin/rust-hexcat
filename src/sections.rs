use crate::error::AppError;
use crate::paint::{PaintOutput, Painter};
use crate::terminal::{Size, Terminal};
use crate::{MessageOrigin, TcpMessage, BUFFER_SIZE};
use error_stack::{IntoReport, Result, ResultExt};
use std::io::Write;
use std::io::{ErrorKind, Read};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::Sender;
use termion::event::Key;

pub(crate) struct Title {
    addr: SocketAddr,
}
impl Title {
    pub(crate) fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}
impl Painter for Title {
    fn paint(&self, size: Size) -> Result<PaintOutput, AppError> {
        let mut output: PaintOutput = Vec::with_capacity(size.height as usize);

        let mut title: Vec<char> = format!(
            "HexCat. Connected to {} (on port {}).",
            self.addr.ip(),
            self.addr.port()
        )
        .chars()
        .collect();
        title.resize(size.width, ' ');
        output.push(title);

        let mut divider: Vec<char> = "────────┬".chars().collect();
        divider.resize(size.width, '─');
        output.push(divider);

        output.resize(size.height, vec![' '; size.width]);
        Ok(output)
    }
}

pub(crate) struct Messages {
    messages: Vec<MessageOrigin>,
    connection: TcpStream,
}
impl Messages {
    pub(crate) fn new(connection: TcpStream) -> Self {
        Self {
            messages: Vec::new(),
            connection,
        }
    }

    pub(crate) fn handle_message(&mut self, message: MessageOrigin) {
        if let MessageOrigin::Local(message) = &message {
            _ = self.connection.write_all(message);
        }
        self.messages.push(message);
    }

    pub(crate) fn scroll(&mut self, key: Key) {
        todo!()
    }

    pub(crate) fn listen(mut connection: TcpStream, sink: Sender<TcpMessage>) {
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut message: Vec<u8> = vec![];
        'connected: loop {
            match connection.read(&mut buffer) {
                Ok(0) => break 'connected,
                Ok(n) => {
                    message.extend_from_slice(&buffer[..n]);
                    _ = sink.send(message.clone());
                    message.truncate(0);
                }
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                Err(_) => break 'connected,
            }
        }
    }
}
impl Painter for Messages {
    fn paint(&self, size: Size) -> Result<PaintOutput, AppError> {
        fn vec_to_line(width: usize, lhs: &str, message: &Vec<u8>, rhs: &str) -> Vec<char> {
            let mut human_readable: String = message
                .iter()
                .map(|byte| format!("{byte:02x} "))
                .collect::<Vec<String>>()
                .join("");
            human_readable.truncate(width - lhs.len() - rhs.len());
            format!("{lhs}{human_readable}{rhs}")
                .chars()
                .collect::<Vec<_>>()
        }

        let mut output: PaintOutput = self
            .messages
            .iter()
            .rev()
            .take(size.height)
            .map(|origin| match origin {
                MessageOrigin::Local(message) => {
                    vec_to_line(size.width, "  LOCAL | ", message, " ")
                }
                MessageOrigin::Remote(message) => {
                    vec_to_line(size.width, " REMOTE | ", message, " ")
                }
            })
            .collect::<Vec<_>>();

        output.resize(size.height, vec![' '; size.width]);
        Ok(output)
    }
}

pub(crate) struct Input {
    input: Vec<char>,
    scroll_offset: usize,
    cursor_position: usize,
}
impl Input {
    pub(crate) fn new() -> Self {
        Self {
            input: Vec::new(),
            scroll_offset: 0,
            cursor_position: 0,
        }
    }

    pub(crate) fn drain_user_message(&mut self) -> Option<TcpMessage> {
        todo!()
    }

    pub(crate) fn handle_key(&mut self, key: Key) {
        match key {
            Key::Left | Key::Right | Key::Home | Key::End => todo!(),
            Key::Char(c) => todo!(),
            Key::Delete => todo!(),
            Key::Backspace => todo!(),
            _ => (),
        }
    }

    pub(crate) fn listen(sink: Sender<Key>) -> Result<(), AppError> {
        loop {
            if let Some(key) = Terminal::read_key()? {
                sink.send(key)
                    .into_report()
                    .attach_printable("Could not communicate user input to main thread.")
                    .change_context(AppError::ChannelBroken)?;
            }
        }
    }
}
impl Painter for Input {
    fn paint(&self, size: Size) -> Result<PaintOutput, AppError> {
        let mut output: PaintOutput = Vec::with_capacity(size.height);

        let mut divider: Vec<char> = "────────┼".chars().collect();
        divider.resize(size.width, '─');
        output.push(divider);

        let prompt = " Input: │ ";
        let max_input_length: usize = size.width - prompt.len() - 1;
        let mut input = self.input.clone();
        input.drain(0..self.scroll_offset);
        input.resize(max_input_length, ' ');

        let mut line: Vec<char> = Vec::new();
        line.extend(prompt.chars());
        line.extend(input);
        output.push(line);

        output.resize(size.height, vec![' '; size.width]);
        Ok(output)
    }
}
