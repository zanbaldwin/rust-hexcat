use crate::error::AppError;
use crate::paint::{Paintable, Painter};
use crate::terminal::Terminal;
use crate::terminal::{Position, Size};
use crate::{sections, MessageOrigin};
use crate::{TcpMessage, THREAD_SLOW_DOWN};
use error_stack::{IntoReport, Result, ResultExt};

use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use termion::event::Key;

struct Sections {
    title: sections::Title,
    messages: sections::Messages,
    input: sections::Input,
}

pub struct WindowReceiver {
    message: Receiver<TcpMessage>,
    input: Receiver<Key>,
}
pub struct Window<'a> {
    terminal: Terminal,
    addr: SocketAddr,
    should_quit: bool,

    receiver: WindowReceiver,
    sections: Sections,
    paintables: Vec<Paintable<'a>>,
}
impl<'a> Window<'a> {
    pub fn new(
        terminal: Terminal,
        connection: TcpStream,
        receiver: WindowReceiver,
    ) -> Result<Self, AppError> {
        let (width, height) = Terminal::size()?;
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

        let mut window = Self {
            should_quit: false,
            terminal,
            addr,
            sections,
            paintables: vec![],
            receiver,
        };
        window.setup_paintables(width, height);
        Ok(window)
    }

    fn setup_paintables(&'a mut self, width: u16, height: u16) {
        self.paintables.push(Paintable {
            painter: &self.sections.input,
            bounds: Size { width, height: 2 },
            position: Position { x: 0, y: 0 },
        });
        self.paintables.push(Paintable {
            painter: &self.sections.messages,
            bounds: Size {
                width,
                height: height - 4,
            },
            position: Position { x: 0, y: 2 },
        });
        self.paintables.push(Paintable {
            painter: &self.sections.input,
            bounds: Size { width, height: 2 },
            position: Position {
                x: 0,
                y: height - 4,
            },
        });
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        Terminal::clear_screen();

        'main: loop {
            if self.should_quit {
                break 'main;
            }

            match self.receiver.message.try_recv() {
                Ok(message) => self
                    .sections
                    .messages
                    .handle_message(MessageOrigin::Remote(message)),
                Err(err) if err == TryRecvError::Empty => (),
                Err(err) => Err(err)
                    .into_report()
                    .attach_printable("TCP thread communication broke.")
                    .change_context(AppError::ChannelBroken)?,
            }

            match self.receiver.input.try_recv() {
                Ok(key) => match key {
                    Key::Up | Key::Down | Key::PageUp | Key::PageDown => {
                        self.sections.messages.scroll(key)
                    }
                    Key::Ctrl('\n') => {
                        if let Some(message) = self.sections.input.drain_user_message() {
                            self.sections
                                .messages
                                .handle_message(MessageOrigin::Local(message));
                        }
                    }
                    _ => self.sections.input.handle_key(key),
                },
                Err(err) if err == TryRecvError::Empty => (),
                Err(err) => Err(err)
                    .into_report()
                    .attach_printable("User Input thread communication broke.")
                    .change_context(AppError::ChannelBroken)?,
            }

            self.draw()?;
            thread::sleep(THREAD_SLOW_DOWN);
        }

        Terminal::clear_screen();
        self.terminal.move_cursor(0, 0);
        Ok(())
    }

    fn draw(&mut self) -> Result<(), AppError> {
        Terminal::cursor_hide();
        for paintable in self.paintables.iter() {
            let content = paintable.paint(
                paintable.bounds().width as usize,
                paintable.bounds().height as usize,
            )?;
        }
        Terminal::cursor_show();
        Terminal::flush()?;
        Ok(())
    }
}
