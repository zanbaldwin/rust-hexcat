use crate::error::AppError;
use crate::paint::{PaintOutput, Painter};
use crate::terminal::{Size, Terminal};
use crate::{MessageOrigin, TcpMessage, BUFFER_SIZE};
use error_stack::{IntoReport, Result, ResultExt};
use std::cmp::min;
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
        let mut output: PaintOutput = Vec::with_capacity(size.height);

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
        fn vec_to_line(width: usize, lhs: &str, message: &[u8], rhs: &str) -> Vec<char> {
            let mut human_readable: String = message
                .iter()
                .map(|byte| format!("{byte:02x} "))
                .collect::<String>();
            human_readable.truncate(width - lhs.len() - rhs.len());
            let mut line = format!("{lhs}{human_readable}{rhs}")
                .chars()
                .collect::<Vec<_>>();
            line.resize(width, ' ');
            line
        }

        let mut output: PaintOutput = self
            .messages
            .iter()
            .rev()
            .take(size.height - 1)
            .rev()
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
}
impl Input {
    pub(crate) fn new() -> Self {
        Self {
            input: Vec::new(),
            prompt: " Input: │ ".to_string(),
        }
    }

    pub(crate) fn drain_user_message(&mut self) -> Option<TcpMessage> {
        let input = self
            .input
            .clone()
            .into_iter()
            .filter(char::is_ascii_hexdigit)
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
        Some(hex)
    }

    pub(crate) fn handle_key(&mut self, key: Key) -> bool {
        match key {
            Key::Char(c) => {
                if c.is_ascii_hexdigit() || c == ' ' {
                    self.input.push(c);
                    return true;
                }
            }
            Key::Backspace => {
                if self.input.pop().is_some() {
                    return true;
                }
            }
            _ => (),
        }
        false
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

    pub(crate) fn get_cursor_x_position(&self, terminal_width: usize) -> u16 {
        let max_input_width = terminal_width - self.prompt.len() - 1;
        (self.prompt.len() + min(self.input.len(), max_input_width) - 2) as u16
    }
}
impl Painter for Input {
    fn paint(&self, size: Size) -> Result<PaintOutput, AppError> {
        let mut output: PaintOutput = Vec::with_capacity(size.height);

        let mut divider: Vec<char> = "────────┼".chars().collect();
        divider.resize(size.width, '─');
        output.push(divider);

        let max_input_length: usize = size.width - self.prompt.len() - 1;
        let mut input = self
            .input
            .iter()
            .rev()
            .take(max_input_length)
            .rev()
            .collect::<Vec<_>>();
        input.resize(max_input_length, &' ');

        let mut line: Vec<char> = Vec::new();
        line.extend(self.prompt.chars());
        line.extend(input);
        output.push(line);

        output.resize(size.height, vec![' '; size.width]);
        Ok(output)
    }
}
