mod crates;
mod display;
mod document;
mod index;

use crate::display::Display;
use crate::document::Document;
use crate::index::Index;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::Event::Key;
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, queue};
use std::io::{stdout, Write};

fn main() {
    //todo handle error
    Display::run().unwrap();
}
