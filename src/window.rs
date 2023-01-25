use crate::terminal::Terminal;
use crate::error::AppError;
use error_stack::{IntoReport, Result, ResultExt};
use std::io::{Read};
use std::io::ErrorKind;
use std::net::{TcpStream, SocketAddr};
use termion::event::Key;

const INITIAL_MESSAGE: &str = "Ctrl-C to Quit, Ctrl-H to enter Hex Mode, Ctrl-A to enter ASCII mode.";
const BUFFER_SIZE: usize = 4_096;

type TcpMessage = Vec<u8>;
enum Sender {
    Local(TcpMessage),
    Remote(TcpMessage),
}

pub struct Window {
    terminal: Terminal,
    connection: TcpStream,
    messages: Vec<Sender>,
    user_input: Vec<u8>,
    pending_nibble: Option<u8>,

    should_quit: bool,

    scroll_offset: usize,
    cursor_position: u16,
}

impl Window {
    pub fn new(terminal: Terminal, connection: TcpStream) -> Self {
        Self {
            should_quit: false,
            terminal,
            connection,
            messages: Vec::new(),
            scroll_offset: 0,
            cursor_position: 0,
            user_input: Vec::new(),
            pending_nibble: None,
        }
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        Terminal::clear_screen();
        self.draw_title()?;
        self.draw_messages_empty()?;
        self.draw_input()?;

        'main: loop {
            if self.should_quit {
                break 'main;
            }
            self.user_input()?;
            self.remote_input()?;
        }

        Terminal::clear_screen();
        self.terminal.move_cursor(0, 0);
        Ok(())
    }

    fn draw(&mut self) -> Result<(), AppError> {
        Terminal::cursor_hide();

        self.draw_title()?;
        self.draw_messages()?;
        let (x, y) = self.draw_input()?;

        self.terminal.move_cursor(x, y);
        Terminal::cursor_show();

        Terminal::flush()?;
        Ok(())
    }

    fn draw_title(&mut self) -> Result<(), AppError> {
        Terminal::cursor_hide();
        let addr: SocketAddr = self.connection.peer_addr()
            .into_report()
            .attach_printable("Could not determine remote client address.")
            .change_context(AppError::StreamRead)?;
        let (width, _height) = Terminal::size()?;
        self.terminal.move_cursor(0, 0);
        let title: String = format!("HexCat. Connected to {} (on port {}).", addr.ip(), addr.port());
        print!("{title}");
        println!("{}\r", " ".repeat((width as usize) - title.len() ));
        let sidebar = "────────┬";
        print!("{sidebar}{}", "─".repeat(width as usize - sidebar.len()));
        Terminal::cursor_show();
        Terminal::flush()?;
        Ok(())
    }

    fn draw_messages(&self) -> Result<(), AppError> {
        Terminal::cursor_hide();
        Terminal::cursor_show();
        Terminal::flush()?;
        Ok(())
        //  LOCAL | 38 64 a6 78 e5 b3 12 dd
        // REMOTE | 9c aa d1 9f 95 37 43 3f 9f ab 01 51 32 a1 33 b8
    }

    fn draw_messages_empty(&mut self) -> Result<(), AppError> {
        Terminal::cursor_hide();
        let (_width, height) = Terminal::size()?;
        for y in 2..height - 2 {
            self.terminal.move_cursor(8, y);
            print!("│");
        }
        Terminal::cursor_show();
        Terminal::flush()?;
        Ok(())
    }

    fn draw_input(&mut self) -> Result<(u16, u16), AppError> {
        Terminal::cursor_hide();
        let (width, height) = Terminal::size()?;

        self.terminal.move_cursor(0, height.saturating_sub(1 + 2 + 1));
        let sidebar = "────────┼";
        println!("{}{}\r", sidebar, "─".repeat((width as usize) - sidebar.len()));

        let prompt = " Input: │ ";
        let max_input_length: usize = (width as usize) - prompt.len() - 1;
        let input: String = self.user_input.iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<String>();
        if let Some(nibble) = self.pending_nibble {
            todo!();
        }

        let range = if max_input_length > input.len() { 0.. } else { input.len() - max_input_length.. };
        let printable_input = match input.get(range) {
            Some(input) => input,
            None => "",
        };

        print!("{prompt}{printable_input}");

        // Bottom two rows.
        // Draw Divider. "───────┼" + "─".fill(terminal_width)
        // Draw Prompt. " Input: │ "
        // Draw User Input. user_input + " "

        Terminal::cursor_show();
        Terminal::flush()?;
        Ok((self.user_input.len() as u16, height.saturating_sub(1)))
    }

    fn user_input(&mut self) -> Result<(), AppError> {
        if let Some(key) = Terminal::read_key_nonblocking()? {
            match key {
                Key::Ctrl('c') => self.should_quit = true,
                Key::Ctrl('\n') => {
                    self.user_input.push('\n' as u8);
                    todo!("Send user input to remote.");
                },
                Key::Up|Key::Down|Key::PageUp|Key::PageDown => self.scroll_offset = {
                    let (_width, height) = Terminal::size()?;
                    match key {
                        Key::Up => self.scroll_offset.saturating_add(1),
                        Key::Down => self.scroll_offset.saturating_sub(1),
                        Key::PageUp => self.scroll_offset.saturating_add(height.saturating_sub(4) as usize),
                        Key::PageDown => self.scroll_offset.saturating_sub(height.saturating_sub(4) as usize),
                        _ => self.scroll_offset,
                    }
                },
                Key::Left|Key::Right|Key::Home|Key::End => todo!(),
                Key::Char(c) => {
                    if c == '\n' {
                        todo!();
                    } else {
                        todo!();
                    }
                },
                Key::Delete => todo!(),
                Key::Backspace => todo!(),
                _ => return Ok(()),
            }
            self.draw_input()?;
        }
        Ok(())
    }

    fn remote_input(&mut self) -> Result<(), AppError> {
        let mut buffer = [0u8; BUFFER_SIZE];
        match self.connection.read(&mut buffer) {
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(error)
                .into_report()
                .attach_printable("Could not read from remote TCP client.")
                .change_context(AppError::StreamRead),
            Ok(0) => Err(AppError::StreamRead)
                .into_report()
                .attach_printable("Remote TCP client closed the connection."),
            Ok(n) => {
                let mut message: TcpMessage = Vec::new();
                message.extend_from_slice(&buffer[..n]);
                self.messages.push(Sender::Remote(message));
                self.draw_messages()?;
                Ok(())
            },
        }
    }
}

// impl Editor {
//     pub fn run(&mut self) {
//         let mut needs_refresh: bool = true;
//         loop {
//             if let Err(error) = self.refresh_screen() {
//                 die(&error);
//             }
//             if self.should_quit {
//                 break;
//             }
//             if let Err(error) = self.process_keypress() {
//                 die(&error);
//             }
//         }
//     }

//     fn refresh_screen(&mut self) -> Result<(), io::Error> {
//         Terminal::cursor_hide();
//         self.terminal.move_pointer(&Position::default(), &Position::default());
//         if self.should_quit {
//             Terminal::clear_screen();
//             println!("Goodbye!\r");
//         } else {
//             self.draw_rows();
//             self.draw_status_bar();
//             self.draw_message_bar();
//             self.terminal.move_pointer(&self.cursor_position, &self.offset);
//         }
//         Terminal::cursor_show();
//         Terminal::flush()
//     }

//     fn process_keypress(&mut self) -> Result<(), io::Error> {
//         let pressed_key = Terminal::read_key()?;
//         match pressed_key {
//             Key::Ctrl('q') => self.should_quit = true,
//             Key::Ctrl('s') => {
//                 if self.document.save().is_ok() {
//                     self.status_message = Some(StatusMessage::from("File saved successfully!".to_string()));
//                 } else {
//                     self.status_message = Some(StatusMessage::from("Error writing to disk.".to_string()));
//                 }
//             },
//             Key::Up|Key::Down|Key::Left|Key::Right|Key::PageUp|Key::PageDown|Key::Home|Key::End => self.move_cursor(pressed_key),
//             Key::Char(c) => {
//                 self.document.insert(&self.cursor_position, c);
//                 if c == '\n' {
//                     self.move_cursor(pressed_key);
//                 } else {
//                     self.move_cursor(Key::Right);
//                 }
//             },
//             Key::Delete => self.document.delete(&self.cursor_position),
//             Key::Backspace => if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
//                 self.move_cursor(Key::Left);
//                 self.document.delete(&self.cursor_position);
//             },
//             _ => (),
//         }
//         self.scroll();
//         Ok(())
//     }

//     fn move_cursor(&mut self, key: Key) {
//         let Position { mut x, mut y} = self.cursor_position;
//         let current_row_width = if let Some(row) = self.document.row(y) { row.len() } else { 0 };

//         let terminal_height = self.terminal.size().height.saturating_sub(1) as usize;
//         let max_height = cmp::max(terminal_height, self.document.len().saturating_sub(1));

//         match key {
//             Key::Up => y = y.saturating_sub(1),
//             Key::Down => {
//                 if y < max_height {
//                     y = y.saturating_add(1);
//                 }
//             },
//             Key::Left => {
//                 if x > 0 {
//                     x -= 1;
//                 } else if y > 0 {
//                     y -= 1;
//                     x = if let Some(row) = self.document.row(y) { row.len() } else { 0 };
//                 } else {
//                     x = 0;
//                 }
//             },
//             Key::Right => if x < current_row_width {
//                 x += 1;
//             } else if y < max_height {
//                 y += 1;
//                 x = 0;
//             },
//             Key::PageUp => y = self.cursor_position.y.saturating_sub(terminal_height),
//             Key::PageDown => y = cmp::min(max_height, self.cursor_position.y.saturating_add(terminal_height)),
//             Key::Home => x = 0,
//             Key::End => x = current_row_width,
//             Key::Char('\n') => {
//                 x = 0;
//                 y = y.saturating_add(1);
//             },
//             _ => (),
//         }
//         let new_row_width = if let Some(row) = self.document.row(y) { row.len() } else { 0 };
//         x = cmp::min(x, new_row_width);
//         self.cursor_position = Position { x, y };
//     }

//     fn scroll(&mut self) {
//         let Position { x, y } = self.cursor_position;
//         let terminal_width = self.terminal.size().width as usize;
//         let terminal_height = self.terminal.size().height as usize;

//         let mut offset = &mut self.offset;

//         if y < offset.y {
//             offset.y = y;
//         } else if y > offset.y.saturating_add(terminal_height.saturating_sub(1)) {
//             offset.y = y.saturating_sub(terminal_height).saturating_add(1);
//         }

//         if x < offset.x {
//             offset.x = x;
//         } else if x >= offset.x.saturating_add(terminal_width) {
//             offset.x = x.saturating_sub(terminal_width).saturating_add(1);
//         }
//     }

//     fn draw_row(&self, row: &Row) {
//         let width = self.terminal.size().width as usize;
//         let start = self.offset.x;
//         let end = self.offset.x + width;
//         let row = row.render(start, end);
//         println!("{}\r", row);
//     }

//     fn draw_rows(&self) {
//         let terminal_height = self.terminal.size().height;
//         for terminal_row in 0..terminal_height {
//             Terminal::clear_current_line();
//             if let Some(row) = self.document.row((terminal_row as usize).saturating_add(self.offset.y)) {
//                 self.draw_row(row);
//             } else if self.document.is_empty() && terminal_row == terminal_height / 3 {
//                 println!("{}\r", left_with_centre_overlay_text(
//                     self.terminal.size().width as usize,
//                     "",
//                     crate::WELCOME_MESSAGE,
//                 ));
//             } else {
//                 println!(" \r");
//             }
//         }
//     }

//     fn draw_status_bar(&self) {
//         let terminal_width = self.terminal.size().width as usize;

//         let mut left = String::from(" Zano");
//         if let Some(filepath) = self.document.filepath() {
//             left.push_str(": ");
//             left.push_str(filepath);
//             if self.document.is_modified() {
//                 left.push_str(" ***");
//             }
//         }
//         left.push(' ');
//         let right = format!(" Line {}:{} ", self.cursor_position.y.saturating_add(1), self.cursor_position.x.saturating_add(1));
//         let mut combined = format!("{}{}{}", left, " ".repeat(terminal_width.saturating_sub(left.len() + right.len())), right);
//         combined.truncate(terminal_width);

//         Terminal::set_bg_colour(STATUS_BG_COLOUR);
//         Terminal::set_fg_colour(STATUS_FG_COLOUR);
//         println!("{}\r", combined);
//         Terminal::reset_fg_colour();
//         Terminal::reset_bg_colour();
//     }

//     fn draw_message_bar(&mut self) {
//         Terminal::clear_current_line();
//         if let Some(message) = &self.status_message {
//             if Instant::now() - message.time < Duration::new(5, 0) {
//                 let mut text = message.text.clone();
//                 text.truncate(self.terminal.size().width as usize);
//                 let padding = (self.terminal.size().width as usize).saturating_sub(text.len()) / 2;
//                 print!("{}{}", " ".repeat(padding), text);
//             } else {
//                 self.status_message = None;
//             }
//         }
//     }
// }

// impl Default for Editor {
//     fn default() -> Self {
//         let args: Vec<String> = env::args().collect();
//         let mut initial_status = String::from(INITIAL_MESSAGE);
//         let document: Document = if args.len() > 1 {
//             let file_name = &args[1];
//             let doc = Document::open(file_name);
//             if let Ok(doc) = doc {
//                 doc
//             } else {
//                 initial_status = format!("ERR: Could not open file: {}", file_name);
//                 Document::default()
//             }
//         } else {
//             Document::default()
//         };
//         Self {
//             should_quit: false,
//             terminal: Terminal::init().expect("Failed to initialize terminal."),
//             cursor_position: Position::default(),
//             offset: Position::default(),
//             document,
//             status_message: Some(StatusMessage::from(initial_status)),
//         }
//     }
// }
