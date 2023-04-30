use std::io::{stdout, Stdout, Write};
use std::ops::Range;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{execute, queue};

use crate::document::Document;

pub struct Display {
    stdout: Stdout,

    document: Document,

    dep_selector: Selector,
    feature_selector: Selector,

    state: DisplayState,
}

impl Display {
    pub fn new() -> anyhow::Result<Display> {
        let document = Document::new("./Cargo.toml")?;

        Ok(Display {
            stdout: stdout(),

            dep_selector: Selector {
                selected: 0,
                length: document.get_deps().len(),
            },
            feature_selector: Selector {
                selected: 0,
                length: 0,
            },

            document,

            state: DisplayState::DepSelect,
        })
    }

    pub fn set_selected_dep(&mut self, dep_name: String) -> anyhow::Result<()> {
        for (index, current_crate) in self.document.get_deps().iter().enumerate() {
            if current_crate.get_name() == dep_name {
                self.dep_selector.selected = index;

                self.selected_dep();
                return Ok(());
            }
        }

        Err(anyhow::Error::msg(format!(
            "dependency \"{}\" could not be found",
            dep_name
        )))
    }

    fn selected_dep(&mut self) {
        self.state = DisplayState::FeatureSelect;

        // set max length to feature count of current feature
        self.feature_selector.length = self
            .document
            .get_dep(self.dep_selector.selected)
            .unwrap()
            .get_features_count();
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        execute!(self.stdout, Hide, Clear(ClearType::All))?;

        loop {
            queue!(self.stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown))?;

            match self.state {
                DisplayState::DepSelect => self.display_deps()?,
                DisplayState::FeatureSelect => self.display_features()?,
            }

            self.stdout.flush()?;

            if self.input_event()? {
                break;
            }
        }

        execute!(self.stdout, Show)?;

        Ok(())
    }

    fn display_deps(&mut self) -> anyhow::Result<()> {
        queue!(self.stdout, Print("Dependencies"))?;

        let dep_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = dep_range.start;

        for dep in &self.document.get_deps()[dep_range] {
            if index == self.dep_selector.selected {
                queue!(self.stdout, MoveTo(0, line_index), Print(">"))?;
            }

            if !dep.has_features() {
                queue!(
                    self.stdout,
                    SetForegroundColor(Color::from((100, 100, 100)))
                )?;
            }

            queue!(
                self.stdout,
                MoveTo(2, line_index),
                Print(dep.get_name()),
                ResetColor
            )?;

            index += 1;
            line_index += 1;
        }

        Ok(())
    }

    fn display_features(&mut self) -> anyhow::Result<()> {
        let deps = self.document.get_deps();
        let dep = deps.get(self.dep_selector.selected).unwrap();

        let feature_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = feature_range.start;

        queue!(
            self.stdout,
            Print(format!("{} {}", dep.get_name(), dep.get_version()))
        )?;

        for (feature_name, data) in &dep.get_features_as_vec()[self.get_max_range()] {
            if data.is_default {
                queue!(self.stdout, SetForegroundColor(Color::Green))?;
            }

            queue!(self.stdout, MoveTo(2, line_index), Print("["))?;

            if data.is_enabled {
                queue!(self.stdout, MoveTo(3, line_index), Print("X"))?;
            }

            queue!(self.stdout, MoveTo(4, line_index), Print("]"))?;
            queue!(self.stdout, ResetColor)?;

            if !dep
                .get_currently_dependent_features(feature_name)
                .is_empty()
            {
                queue!(
                    self.stdout,
                    SetForegroundColor(Color::from((100, 100, 100)))
                )?;
            }

            queue!(self.stdout, MoveTo(6, line_index), Print(feature_name))?;
            queue!(self.stdout, ResetColor)?;

            if index == self.feature_selector.selected {
                queue!(self.stdout, MoveTo(0, line_index), Print(">"))?;

                let sub_features = &data.sub_features;

                if !sub_features.is_empty() {
                    line_index += 1;

                    queue!(self.stdout, MoveTo(6, line_index), Print("└"))?;

                    let mut sub_features_str = "".to_string();

                    for sub_feature in sub_features {
                        sub_features_str += sub_feature;
                        sub_features_str += " ";
                    }

                    queue!(self.stdout, MoveTo(8, line_index), Print(sub_features_str))?;
                }
            }

            line_index += 1;
            index += 1;
        }

        Ok(())
    }

    fn input_event(&mut self) -> anyhow::Result<bool> {
        if let Event::Key(key_event) = read()? {
            if let KeyEventKind::Press = key_event.kind {
                match key_event.code {
                    KeyCode::Up => match self.state {
                        DisplayState::DepSelect => {
                            self.dep_selector.shift(-1);
                        }
                        DisplayState::FeatureSelect => {
                            self.feature_selector.shift(-1);
                        }
                    },
                    KeyCode::Down => match self.state {
                        DisplayState::DepSelect => {
                            self.dep_selector.shift(1);
                        }
                        DisplayState::FeatureSelect => {
                            self.feature_selector.shift(1);
                        }
                    },
                    KeyCode::Char(' ') | KeyCode::Enter => match self.state {
                        DisplayState::DepSelect => {
                            if self
                                .document
                                .get_dep(self.dep_selector.selected)?
                                .has_features()
                            {
                                self.selected_dep();

                                //needed to wrap
                                self.feature_selector.shift(0);
                            }
                        }
                        DisplayState::FeatureSelect => {
                            let dep = self
                                .document
                                .get_deps_mut()
                                .get_mut(self.dep_selector.selected)
                                .unwrap();

                            //todo better
                            let feature_name = dep
                                .get_features_as_vec()
                                .get(self.feature_selector.selected)
                                .unwrap()
                                .0
                                .clone();

                            dep.toggle_feature_usage(&feature_name);

                            self.document.write_dep(self.dep_selector.selected);
                        }
                    },
                    KeyCode::Backspace => match self.state {
                        DisplayState::DepSelect => {
                            return Ok(true);
                        }
                        DisplayState::FeatureSelect => {
                            self.state = DisplayState::DepSelect;
                        }
                    },
                    KeyCode::Char('q') => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    fn get_max_range(&self) -> Range<usize> {
        let current_selected = match self.state {
            DisplayState::DepSelect => self.dep_selector.selected,
            DisplayState::FeatureSelect => self.feature_selector.selected,
        } as isize;

        let max_range = match self.state {
            DisplayState::DepSelect => self.document.get_deps().len(),
            DisplayState::FeatureSelect => self
                .document
                .get_dep(self.dep_selector.selected)
                .unwrap()
                .get_features_count(),
        };

        let mut offset = 0;

        if let DisplayState::FeatureSelect = self.state {
            let current_crate = self.document.get_dep(self.dep_selector.selected).unwrap();

            let features = current_crate.get_features_as_vec();

            let (_, data) = features.get(self.feature_selector.selected).unwrap();

            if !data.sub_features.is_empty() {
                offset = 1;
            }
        }

        let height = size().unwrap().1 as usize;

        let start = (current_selected - height as isize / 2 + 1)
            .min(max_range as isize - height as isize + 1 + offset as isize)
            .max(0) as usize;

        start..max_range.min(start + height - 1 - offset)
    }
}

enum DisplayState {
    DepSelect,
    FeatureSelect,
}

struct Selector {
    selected: usize,
    length: usize,
}

impl Selector {
    fn shift(&mut self, shift: isize) {
        let mut selected_temp = self.selected as isize;

        selected_temp += self.length as isize;
        selected_temp += shift;

        selected_temp %= self.length as isize;

        self.selected = selected_temp as usize;
    }
}
