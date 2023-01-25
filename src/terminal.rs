use crate::error::AppError;
use error_stack::{IntoReport, Result, ResultExt};
use std::io;
use std::io::Write;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

#[derive(Default, Clone)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}
#[derive(Default, Clone)]
pub struct Size {
    pub width: u16,
    pub height: u16,
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

    pub fn size() -> Result<(u16, u16), AppError> {
        let size = termion::terminal_size()
            .into_report()
            .attach_printable("Could not determine terminal size.")
            .change_context(AppError::TerminalError)?;
        Ok(size)
    }

    pub fn clear_screen() {
        print!("{}", termion::clear::All);
    }

    pub fn clear_current_line() {
        print!("{}", termion::clear::CurrentLine);
    }

    pub fn move_cursor(&mut self, x: u16, y: u16) {
        self.cursor = Position { x, y };
        print!(
            "{}",
            termion::cursor::Goto(
                self.cursor.x.saturating_add(1),
                self.cursor.y.saturating_add(1),
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

    pub fn set_bg_colour(colour: color::Rgb) {
        print!("{}", color::Bg(colour));
    }

    pub fn reset_bg_colour() {
        print!("{}", color::Bg(color::Reset));
    }

    pub fn set_fg_colour(color: color::Rgb) {
        print!("{}", color::Fg(color));
    }

    pub fn reset_fg_colour() {
        print!("{}", color::Fg(color::Reset));
    }

    pub fn reset_colour() {
        Self::reset_bg_colour();
        Self::reset_fg_colour();
    }
}
