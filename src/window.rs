use crate::error::AppError;
use crate::paint::Painter;
use crate::terminal::Position;
use crate::terminal::Size;
use crate::terminal::Terminal;
use crate::{sections, MessageOrigin};
use crate::{TcpMessage, THREAD_SLOW_DOWN};
use error_stack::{IntoReport, Result, ResultExt};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use termion::event::Key;

struct Sections {
    title: sections::Title,
    messages: sections::Messages,
    input: sections::Input,
}

pub(crate) struct WindowReceiver {
    message: Receiver<TcpMessage>,
    input: Receiver<Key>,
}
impl WindowReceiver {
    pub(crate) fn new(message: Receiver<TcpMessage>, input: Receiver<Key>) -> Self {
        Self { message, input }
    }
}
pub(crate) struct Window {
    terminal: Terminal,
    should_quit: bool,
    receiver: WindowReceiver,
    sections: Sections,
}
impl Window {
    pub(crate) fn new(
        terminal: Terminal,
        connection: TcpStream,
        receiver: WindowReceiver,
    ) -> Result<Self, AppError> {
        let addr = connection
            .peer_addr()
            .into_report()
            .attach_printable("Could not determine address of remote connection.")
            .change_context(AppError::StreamRead)?;
        let sections = Sections {
            title: sections::Title::new(addr),
            messages: sections::Messages::new(connection),
            input: sections::Input::new(),
        };

        let window = Self {
            should_quit: false,
            terminal,
            sections,
            receiver,
        };

        Ok(window)
    }

    pub(crate) fn run(&mut self) -> Result<(), AppError> {
        Terminal::clear_screen();

        let mut should_draw = true;

        'main: loop {
            if self.should_quit {
                break 'main;
            }

            match self.receiver.message.try_recv() {
                Ok(message) => {
                    self.sections
                        .messages
                        .handle_message(MessageOrigin::Remote(message));
                    should_draw = true;
                }
                Err(err) if err == TryRecvError::Empty => (),
                Err(err) => Err(err)
                    .into_report()
                    .attach_printable("TCP thread communication broke.")
                    .change_context(AppError::ChannelBroken)?,
            }

            match self.receiver.input.try_recv() {
                Ok(key) => {
                    match key {
                        Key::Ctrl('c') => {
                            self.should_quit = true;
                        }
                        Key::Char('\n') => {
                            if let Some(message) = self.sections.input.drain_user_message() {
                                self.sections
                                    .messages
                                    .handle_message(MessageOrigin::Local(message));
                                should_draw = true;
                            }
                        }
                        _ => should_draw = self.sections.input.handle_key(key),
                    };
                }
                Err(err) if err == TryRecvError::Empty => (),
                Err(err) => Err(err)
                    .into_report()
                    .attach_printable("User Input thread communication broke.")
                    .change_context(AppError::ChannelBroken)?,
            }

            if should_draw {
                self.draw()?;
                should_draw = false;
            }
            thread::sleep(THREAD_SLOW_DOWN);
        }

        Terminal::clear_screen();
        self.terminal.move_cursor(0, 0);
        Ok(())
    }

    fn draw(&mut self) -> Result<(), AppError> {
        Terminal::cursor_hide();
        let terminal_size: Size = Terminal::size()?;

        self.print(
            &self.sections.title.paint(Size {
                width: terminal_size.width,
                height: 2,
            })?,
            Position { x: 0, y: 0 },
        );

        self.print(
            &self.sections.messages.paint(Size {
                width: terminal_size.width,
                height: terminal_size.height - 4,
            })?,
            Position { x: 0, y: 2 },
        );

        self.print(
            &self.sections.input.paint(Size {
                width: terminal_size.width,
                height: 2,
            })?,
            Position {
                x: 0,
                y: terminal_size.height - 3,
            },
        );

        self.terminal.move_cursor(
            self.sections
                .input
                .get_cursor_x_position(terminal_size.width),
            (terminal_size.height - 2) as u16,
        );

        Terminal::cursor_show();
        Terminal::flush()?;
        Ok(())
    }

    fn print(&mut self, content: &[Vec<char>], position: Position) {
        content.iter().enumerate().for_each(|(index, line)| {
            self.terminal
                .move_cursor(position.x as u16, (position.y + index) as u16);
            print!("{}", line.iter().collect::<String>());
        });
    }
}
