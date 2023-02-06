use crate::error::AppError;
use error_stack::{IntoReport, Result, ResultExt};
use std::io;
use std::io::Write;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

#[derive(Default, Clone, Copy)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

pub struct Terminal {
    _stdout: RawTerminal<io::Stdout>,
    cursor: Position,
}

impl Terminal {
    pub fn init() -> Result<Self, AppError> {
        Ok(Self {
            _stdout: io::stdout()
                .into_raw_mode()
                .into_report()
                .attach_printable("Could not enter RAW mode.")
                .change_context(AppError::TerminalError)?,
            cursor: Position::default(),
        })
    }

    pub fn size() -> Result<Size, AppError> {
        let (width, height) = termion::terminal_size()
            .into_report()
            .attach_printable("Could not determine terminal size.")
            .change_context(AppError::TerminalError)?;
        Ok(Size {
            width: width as usize,
            height: height as usize,
        })
    }

    pub fn clear_screen() {
        print!("{}", termion::clear::All);
    }

    pub fn move_cursor(&mut self, x: u16, y: u16) {
        self.cursor = Position {
            x: x as usize,
            y: y as usize,
        };
        print!(
            "{}",
            termion::cursor::Goto(
                self.cursor.x.saturating_add(1) as u16,
                self.cursor.y.saturating_add(1) as u16,
            )
        );
    }

    pub fn cursor_hide() {
        print!("{}", termion::cursor::Hide);
    }

    pub fn cursor_show() {
        print!("{}", termion::cursor::Show);
    }

    pub fn flush() -> Result<(), AppError> {
        io::stdout()
            .flush()
            .into_report()
            .attach_printable("Could not flush display buffer to TTY.")
            .change_context(AppError::TerminalError)?;
        Ok(())
    }

    pub fn read_key() -> Result<Option<Key>, AppError> {
        if let Some(key) = io::stdin().lock().keys().next() {
            match key {
                Ok(key) => Ok(Some(key)),
                Err(error) => Err(error)
                    .into_report()
                    .attach_printable("Could not determine user input.")
                    .change_context(AppError::UserInput),
            }
        } else {
            Ok(None)
        }
    }
}
