use std::io::{stdout, Stdout, Write};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, queue};

use crate::document::Document;
use crate::index::Index;

pub struct Display {
    document: Document,
    stdout: Stdout,

    crate_selected: usize,
    feature_selected: usize,

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

            crate_selected: 0,
            feature_selected: 0,

            state: DisplayState::CrateSelect,
        };

        display.start()
    }

    fn start(&mut self) -> anyhow::Result<()> {
        loop {
            queue!(self.stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown))?;

            match self.state {
                DisplayState::CrateSelect => self.render_crate_select()?,
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

    fn render_crate_select(&mut self) -> anyhow::Result<()> {
        //todo handle no deps

        queue!(self.stdout, Print("Dependencies"))?;

        for (index, dep) in self.document.get_deps().iter().enumerate() {
            if index == self.crate_selected {
                queue!(self.stdout, MoveTo(0, index as u16 + 1), Print(">"))?;
            }

            queue!(
                self.stdout,
                MoveTo(2, index as u16 + 1),
                Print(dep.get_name())
            )?;
        }

        Ok(())
    }

    fn render_feature_select(&mut self, dep_index: usize) -> anyhow::Result<()> {
        //todo handle no feature

        let deps = self.document.get_deps();
        let dep = deps.get(dep_index).unwrap();

        let mut line_index = 1;

        queue!(
            self.stdout,
            Print(format!("{} {}", dep.get_name(), dep.get_version()))
        )?;

        for (index, (feature_name, active)) in dep.get_features().iter().enumerate() {
            if dep.is_default_feature(feature_name) {
                queue!(self.stdout, SetForegroundColor(Color::Green))?;
            }

            queue!(self.stdout, MoveTo(2, line_index), Print("["))?;

            if *active {
                queue!(self.stdout, MoveTo(3, line_index), Print("X"))?;
            }

            queue!(self.stdout, MoveTo(4, line_index), Print("]"))?;
            queue!(self.stdout, ResetColor)?;
            queue!(self.stdout, MoveTo(6, line_index), Print(feature_name))?;

            if index == self.feature_selected {
                queue!(self.stdout, MoveTo(0, line_index), Print(">"))?;

                let sub_features = dep.get_sub_features(feature_name);

                if sub_features.len() > 0 {
                    line_index += 1;

                    queue!(self.stdout, MoveTo(5, line_index), Print("↳"))?;

                    let mut sub_features_str = "".to_string();

                    for sub_feature in sub_features {
                        sub_features_str += &sub_feature;
                        sub_features_str += " ";
                    }

                    queue!(self.stdout, MoveTo(7, line_index), Print(sub_features_str))?;
                }
            }

            line_index += 1;
        }

        Ok(())
    }

    fn input_event(&mut self) -> anyhow::Result<bool> {
        match read()? {
            Event::Key(key_event) => {
                if let KeyEventKind::Press = key_event.kind {
                    match key_event.code {
                        KeyCode::Up => match self.state {
                            DisplayState::CrateSelect => {
                                self.shift_selection(self.document.get_deps().len(), -1);
                            }
                            DisplayState::FeatureSelect(dep_index) => {
                                let max_length =
                                    self.document.get_dep(dep_index)?.get_features_count();

                                self.shift_selection(max_length, -1);
                            }
                        },
                        KeyCode::Down => match self.state {
                            DisplayState::CrateSelect => {
                                self.shift_selection(self.document.get_deps().len(), 1);
                            }
                            DisplayState::FeatureSelect(dep_index) => {
                                let max_length =
                                    self.document.get_dep(dep_index)?.get_features_count();

                                self.shift_selection(max_length, 1);
                            }
                        },
                        KeyCode::Char(' ') | KeyCode::Enter => match self.state {
                            DisplayState::CrateSelect => {
                                self.state = DisplayState::FeatureSelect(self.crate_selected);

                                let max_length = self
                                    .document
                                    .get_dep(self.crate_selected)?
                                    .get_features_count();

                                self.shift_selection(max_length, 0);
                            }
                            DisplayState::FeatureSelect(dep_index) => {
                                self.document
                                    .get_deps_mut()
                                    .get_mut(dep_index)
                                    .unwrap()
                                    .toggle_feature_usage(self.feature_selected);

                                self.document.write_dep(dep_index);
                            }
                        },
                        KeyCode::Backspace => match self.state {
                            DisplayState::CrateSelect => {
                                return Ok(true);
                            }
                            DisplayState::FeatureSelect(_) => {
                                self.state = DisplayState::CrateSelect;
                            }
                        },
                        KeyCode::Char('q') => {
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn shift_selection(&mut self, max_length: usize, shift: isize) {
        let mut selected_temp;

        match self.state {
            DisplayState::CrateSelect => selected_temp = self.crate_selected as isize,
            DisplayState::FeatureSelect(..) => selected_temp = self.feature_selected as isize,
        }

        selected_temp += max_length as isize;
        selected_temp += shift;

        selected_temp %= max_length as isize;

        match self.state {
            DisplayState::CrateSelect => self.crate_selected = selected_temp as usize,
            DisplayState::FeatureSelect(..) => self.feature_selected = selected_temp as usize,
        }
    }
}

enum DisplayState {
    CrateSelect,
    FeatureSelect(usize),
}
