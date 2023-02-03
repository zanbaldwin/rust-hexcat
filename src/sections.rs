use crate::error::AppError;
use crate::paint::{PaintOutput, Painter};
use crate::terminal::{Size, Terminal};
use crate::{MessageOrigin, TcpMessage, BUFFER_SIZE};
use error_stack::{IntoReport, Result, ResultExt};
use std::io::Write;
use std::io::{ErrorKind, Read};
use std::net::{SocketAddr, TcpStream};
use std::num::ParseIntError;
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
        match key {
            Key::Up => todo!(),
            Key::PageUp => todo!(),
            Key::Down => todo!(),
            Key::PageDown => todo!(),
            _ => {}
        };
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
                    vec_to_line(size.width, "  LOCAL │ ", message, " ")
                }
                MessageOrigin::Remote(message) => {
                    vec_to_line(size.width, " REMOTE │ ", message, " ")
                }
            })
            .collect::<Vec<_>>();

        let mut empty_line: Vec<char> = "        │".chars().collect();
        empty_line.resize(size.width, ' ');
        output.resize(size.height, empty_line);
        Ok(output)
    }
}

pub(crate) struct Input {
    input: Vec<char>,
    prompt: String,
    scroll_offset: usize,
    cursor_position: usize,
}
impl Input {
    pub(crate) fn new() -> Self {
        Self {
            input: Vec::new(),
            prompt: " Input: │ ".to_string(),
            scroll_offset: 0,
            cursor_position: 0,
        }
    }

    pub(crate) fn drain_user_message(&mut self) -> Option<TcpMessage> {
        let input = self
            .input
            .clone()
            .into_iter()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<Vec<char>>();
        if input.len() % 2 != 0 {
            return None;
        }

        let hex = input
            .chunks(2)
            .map(|double_hex_chars| double_hex_chars.iter().collect::<String>())
            .filter_map(|hex_string| u8::from_str_radix(&hex_string, 16).ok())
            .collect::<Vec<_>>();
        self.input.truncate(0);
        self.cursor_position = 0;
        self.scroll_offset = 0;
        Some(hex)
    }

    pub(crate) fn handle_key(&mut self, key: Key) {
        match key {
            Key::Left | Key::Right | Key::End => todo!(),
            Key::Home => {
                self.cursor_position = 0;
                self.scroll_offset = 0;
            }
            Key::Char(c) => {
                if c.is_ascii_hexdigit() {
                    self.input.push(c);
                    self.cursor_position += 1;
                } else if c.is_whitespace() {
                    self.input.push(' ');
                    self.cursor_position += 1;
                }
            }
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

    pub(crate) fn get_cursor_x_position(&self) -> u16 {
        (self.prompt.len() + self.cursor_position - 2) as u16
    }
}
impl Painter for Input {
    fn paint(&self, size: Size) -> Result<PaintOutput, AppError> {
        let mut output: PaintOutput = Vec::with_capacity(size.height);

        let mut divider: Vec<char> = "────────┼".chars().collect();
        divider.resize(size.width, '─');
        output.push(divider);

        let max_input_length: usize = size.width - self.prompt.len() - 1;
        let mut input = self.input.clone();
        input.drain(0..self.scroll_offset);
        input.resize(max_input_length, ' ');

        let mut line: Vec<char> = Vec::new();
        line.extend(self.prompt.chars());
        line.extend(input);
        output.push(line);

        output.resize(size.height, vec![' '; size.width]);
        Ok(output)
    }
}
