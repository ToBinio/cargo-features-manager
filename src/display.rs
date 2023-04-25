use crate::document::Document;
use crate::index::Index;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, queue};
use std::io::{stdout, Stdout, Write};

pub struct Display {
    document: Document,
    stdout: Stdout,

    selected: usize,
    state: DisplayState,
}

impl Display {
    pub fn run() -> anyhow::Result<()> {
        let document = Document::new("./Cargo.toml", Index::new());

        let mut stdout = stdout();
        execute!(stdout, Hide, Clear(ClearType::All))?;

        let mut display = Display {
            document,
            stdout,

            selected: 0,
            state: DisplayState::DepSelect,
        };

        display.start()
    }

    fn start(&mut self) -> anyhow::Result<()> {
        loop {
            queue!(self.stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown))?;

            match self.state {
                DisplayState::DepSelect => self.render_dep_select()?,
                DisplayState::FeatureSelect(dep_index) => self.render_feature_select(dep_index)?,
            }

            self.stdout.flush()?;

            if self.input_event()? {
                break;
            }
        }

        execute!(self.stdout, Show)?;

        Ok(())
    }

    fn render_dep_select(&mut self) -> anyhow::Result<()> {
        for (index, dep) in self.document.get_deps().iter().enumerate() {
            if index == self.selected {
                queue!(self.stdout, MoveTo(0, index as u16), Print(">"))?;
            }

            queue!(self.stdout, MoveTo(2, index as u16), Print(dep.get_name()))?;
        }

        Ok(())
    }

    fn render_feature_select(&mut self, dep_index: usize) -> anyhow::Result<()> {
        let deps = self.document.get_deps();
        let dep = deps.get(dep_index).unwrap();

        for (index, (feature_name, active)) in dep.get_unique_features().iter().enumerate() {
            if index == self.selected {
                queue!(self.stdout, MoveTo(0, index as u16), Print(">"))?;
            }

            queue!(self.stdout, MoveTo(2, index as u16), Print("["))?;

            if *active {
                queue!(self.stdout, MoveTo(3, index as u16), Print("X"))?;
            }

            queue!(self.stdout, MoveTo(4, index as u16), Print("]"))?;
            queue!(self.stdout, MoveTo(5, index as u16), Print(feature_name))?;
        }

        Ok(())
    }

    fn input_event(&mut self) -> anyhow::Result<bool> {
        match read()? {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Key(key_event) => {
                if let KeyEventKind::Press = key_event.kind {
                    match key_event.code {
                        KeyCode::Up => match self.state {
                            DisplayState::DepSelect => {
                                self.shift_selection(self.document.get_deps().len(), -1);
                            }
                            DisplayState::FeatureSelect(dep_index) => {
                                let max_length = self
                                    .document
                                    .get_deps()
                                    .get(dep_index)
                                    .unwrap()
                                    .get_unique_features()
                                    .len();

                                self.shift_selection(max_length, -1);
                            }
                        },
                        KeyCode::Down => match self.state {
                            DisplayState::DepSelect => {
                                self.shift_selection(self.document.get_deps().len(), 1);
                            }
                            DisplayState::FeatureSelect(dep_index) => {
                                let max_length = self
                                    .document
                                    .get_deps()
                                    .get(dep_index)
                                    .unwrap()
                                    .get_unique_features()
                                    .len();

                                self.shift_selection(max_length, 1);
                            }
                        },
                        KeyCode::Char(char) => {
                            if char == 'q' {
                                return Ok(true);
                            }
                        }
                        KeyCode::Enter => match self.state {
                            DisplayState::DepSelect => {
                                self.state = DisplayState::FeatureSelect(self.selected);
                                self.selected = 0
                            }
                            DisplayState::FeatureSelect(dep_index) => {
                                self.document
                                    .get_deps_mut()
                                    .get_mut(dep_index)
                                    .unwrap()
                                    .toggle_feature_usage(self.selected);

                                self.document.write_dep(dep_index);
                            }
                        },
                        KeyCode::Backspace => match self.state {
                            DisplayState::DepSelect => {
                                return Ok(true);
                            }
                            DisplayState::FeatureSelect(_) => {
                                self.state = DisplayState::DepSelect;
                                self.selected = 0
                            }
                        },
                        _ => {}
                    }
                }
            }
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }

        Ok(false)
    }

    fn shift_selection(&mut self, max_length: usize, shift: isize) {
        let mut selected_temp = self.selected as isize;

        selected_temp += max_length as isize;
        selected_temp += shift;

        selected_temp %= max_length as isize;

        self.selected = selected_temp as usize;
    }
}

enum DisplayState {
    DepSelect,
    FeatureSelect(usize),
}
