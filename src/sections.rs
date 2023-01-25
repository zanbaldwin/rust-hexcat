use crate::error::AppError;
use crate::paint::{PaintOutput, Painter};
use crate::terminal::Terminal;
use crate::{MessageOrigin, TcpMessage, BUFFER_SIZE};
use error_stack::{IntoReport, Result, ResultExt};
use std::io::Write;
use std::io::{ErrorKind, Read};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::Sender;
use termion::event::Key;

pub struct Title {
    addr: SocketAddr,
}
impl Title {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}
impl Painter for Title {
    fn paint(&self, width: usize, height: usize) -> Result<PaintOutput, AppError> {
        let mut output: PaintOutput = Vec::with_capacity(height);

        let mut title: Vec<char> = format!(
            "HexCat. Connected to {} (on port {}).",
            self.addr.ip(),
            self.addr.port()
        )
        .chars()
        .collect();
        title.resize(width, ' ');
        output.push(title);

        let mut divider: Vec<char> = "────────┬".chars().collect();
        divider.resize(width, '─');
        output.push(divider);

        output.resize(height, vec![' '; width]);
        Ok(output)
    }
}

pub struct Messages {
    messages: Vec<MessageOrigin>,
    connection: TcpStream,
}
impl Messages {
    pub fn new(connection: TcpStream) -> Self {
        Self {
            messages: Vec::new(),
            connection,
        }
    }

    pub fn handle_message(&mut self, message: MessageOrigin) {
        if let MessageOrigin::Local(message) = &message {
            _ = self.connection.write_all(message);
        }
        self.messages.push(message);
    }

    pub fn scroll(&mut self, key: Key) {
        todo!()
    }

    pub fn listen(mut connection: TcpStream, sink: Sender<TcpMessage>) {
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut message: Vec<u8> = vec![];
        'connected: loop {
            match connection.read(&mut buffer) {
                Ok(0) => break 'connected,
                Ok(n) => {
                    message.extend_from_slice(&buffer[..n]);
                    sink.send(message.clone());
                    message.truncate(0);
                }
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                Err(_) => break 'connected,
            }
        }
    }
}
impl Painter for Messages {
    fn paint(&self, width: usize, height: usize) -> Result<PaintOutput, AppError> {
        //  LOCAL | 38 64 a6 78 e5 b3 12 dd
        // REMOTE | 9c aa d1 9f 95 37 43 3f 9f ab 01 51 32 a1 33 b8

        let mut output: PaintOutput = Vec::with_capacity(height);
        output.resize(height, vec![' '; width]);
        Ok(output)
    }
}

pub struct Input {
    input: Vec<char>,
    scroll_offset: usize,
    cursor_position: usize,
}
impl Input {
    pub fn new() -> Self {
        Self {
            input: Vec::new(),
            scroll_offset: 0,
            cursor_position: 0,
        }
    }

    pub fn drain_user_message(&mut self) -> Option<TcpMessage> {
        todo!()
    }

    pub fn handle_key(&mut self, key: Key) {
        match key {
            Key::Left | Key::Right | Key::Home | Key::End => todo!(),
            Key::Char(c) => todo!(),
            Key::Delete => todo!(),
            Key::Backspace => todo!(),
            _ => (),
        }
    }

    pub fn listen(sink: Sender<Key>) -> Result<(), AppError> {
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
    fn paint(&self, width: usize, height: usize) -> Result<PaintOutput, AppError> {
        let mut output: PaintOutput = Vec::with_capacity(height);

        let mut divider: Vec<char> = "────────┼".chars().collect();
        divider.resize(width, '─');
        output.push(divider);

        let prompt = " Input: │ ";
        let max_input_length: usize = width - prompt.len() - 1;
        let mut input = self.input.clone();
        input.drain(0..self.scroll_offset);
        input.resize(max_input_length, ' ');

        let mut line: Vec<char> = Vec::new();
        line.extend(prompt.chars());
        line.extend(input);
        output.push(line);

        output.resize(height, vec![' '; width]);
        Ok(output)
    }
}
