mod crates;
mod display;
mod document;
mod index;

use crate::display::Display;

fn main() {
    //todo handle error
    Display::run().unwrap();
}
