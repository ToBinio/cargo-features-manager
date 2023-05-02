use std::io::{stdout, Stdout, Write};
use std::ops::Range;

use crossterm::cursor::{Hide, MoveTo, RestorePosition, SavePosition, SetCursorStyle, Show};
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{execute, queue};

use crate::document::Document;

pub struct Display {
    stdout: Stdout,

    document: Document,

    dep_selector: Selector<usize>,
    feature_selector: Selector<String>,

    state: DisplayState,

    is_type_mode: bool,
    search_text: String,
}

impl Display {
    pub fn new() -> anyhow::Result<Display> {
        let document = Document::new("./Cargo.toml")?;

        Ok(Display {
            stdout: stdout(),

            dep_selector: Selector {
                selected: 0,
                data: document.get_deps_filtered_view("".to_string()),
            },

            feature_selector: Selector {
                selected: 0,
                data: vec![],
            },

            document,

            state: DisplayState::DepSelect,
            is_type_mode: false,
            search_text: "".to_string(),
        })
    }

    pub fn set_selected_dep(&mut self, dep_name: String) -> anyhow::Result<()> {
        match self.document.get_dep_index(&dep_name) {
            Ok(index) => {
                self.dep_selector.selected = index;

                self.selected_dep();
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn selected_dep(&mut self) {
        self.state = DisplayState::FeatureSelect;

        let dep = self
            .document
            .get_dep(*self.dep_selector.get_selected().unwrap())
            .unwrap();

        // update selector
        self.feature_selector.data = dep.get_features_filtered_view(self.search_text.clone());
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        execute!(self.stdout, Hide, Clear(ClearType::All))?;

        loop {
            queue!(
                self.stdout,
                MoveTo(0, 0),
                Clear(ClearType::FromCursorDown),
                Hide
            )?;

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
        queue!(self.stdout, Print("Dependencies"), SavePosition)?;

        let dep_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = dep_range.start;

        for dep in &self.dep_selector.data[dep_range] {
            let dep = self.document.get_dep(*dep).unwrap();

            if index == self.dep_selector.selected && !self.is_type_mode {
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

        self.display_type_mode()?;

        Ok(())
    }

    fn display_features(&mut self) -> anyhow::Result<()> {
        let dep = self
            .document
            .get_dep(*self.dep_selector.get_selected().unwrap())
            .unwrap();

        let feature_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = feature_range.start;

        queue!(
            self.stdout,
            Print(format!("{} {}", dep.get_name(), dep.get_version())),
            SavePosition
        )?;

        for feature_name in &self.feature_selector.data[self.get_max_range()] {
            let data = dep.get_feature(feature_name);

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

            if index == self.feature_selector.selected && !self.is_type_mode {
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

        self.display_type_mode()?;

        Ok(())
    }

    /// crossterm::SavePosition has be been set before calling this function
    fn display_type_mode(&mut self) -> anyhow::Result<()> {
        if !self.search_text.is_empty() || self.is_type_mode {
            queue!(
                self.stdout,
                RestorePosition,
                Print(format!(" - {}", self.search_text)),
            )?;
        }

        if self.is_type_mode {
            queue!(self.stdout, Show, SetCursorStyle::BlinkingBar)?;
        }

        Ok(())
    }

    fn input_event(&mut self) -> anyhow::Result<bool> {
        if let Event::Key(key_event) = read()? {
            if let KeyEventKind::Press = key_event.kind {
                match (key_event.code, &self.state, self.is_type_mode) {
                    //movement
                    //up
                    (KeyCode::Up, DisplayState::DepSelect, false) => {
                        self.dep_selector.shift(-1);
                    }
                    (KeyCode::Up, DisplayState::FeatureSelect, false) => {
                        if self.feature_selector.has_data() {
                            self.feature_selector.shift(-1);
                        }
                    }
                    //down
                    (KeyCode::Down, DisplayState::DepSelect, false) => {
                        self.dep_selector.shift(1);
                    }
                    (KeyCode::Down, DisplayState::FeatureSelect, false) => {
                        if self.feature_selector.has_data() {
                            self.feature_selector.shift(1);
                        }
                    }

                    //selection
                    (KeyCode::Enter, DisplayState::DepSelect, false)
                    | (KeyCode::Char(' '), DisplayState::DepSelect, false) => {
                        if self.dep_selector.has_data() {
                            self.search_text = "".to_string();

                            if self
                                .document
                                .get_dep(*self.dep_selector.get_selected().unwrap())?
                                .has_features()
                            {
                                self.selected_dep();

                                //needed to wrap
                                self.feature_selector.shift(0);
                            }
                        }
                    }
                    (KeyCode::Enter, DisplayState::FeatureSelect, false)
                    | (KeyCode::Char(' '), DisplayState::FeatureSelect, false) => {
                        if self.feature_selector.has_data() {
                            let dep = self
                                .document
                                .get_dep_mut(*self.dep_selector.get_selected().unwrap());

                            dep.toggle_feature_usage(self.feature_selector.get_selected().unwrap());

                            self.document.write_dep(self.dep_selector.selected);
                        }
                    }

                    //typing
                    (KeyCode::Char('s'), _, false) => {
                        self.is_type_mode = true;

                        match self.state {
                            DisplayState::DepSelect => {
                                self.dep_selector.selected = 0;
                            }
                            DisplayState::FeatureSelect => {
                                self.feature_selector.selected = 0;
                            }
                        }
                    }
                    (KeyCode::Char('r'), _, false) => {
                        self.search_text = "".to_string();

                        self.update_selected_data();
                    }
                    (KeyCode::Enter, _, true) => {
                        self.is_type_mode = false;
                    }
                    (KeyCode::Char(char), _, true) => {
                        self.search_text += char.to_string().as_str();

                        self.update_selected_data();
                    }
                    (KeyCode::Backspace, _, true) => {
                        let _ = self.search_text.pop();

                        self.update_selected_data();
                    }

                    //quit
                    (KeyCode::Char('q'), _, false) => {
                        return Ok(true);
                    }

                    //back
                    (KeyCode::Backspace, DisplayState::DepSelect, false) => {
                        return Ok(true);
                    }
                    (KeyCode::Backspace, DisplayState::FeatureSelect, false) => {
                        self.search_text = "".to_string();

                        self.state = DisplayState::DepSelect;

                        self.update_selected_data();
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
            DisplayState::DepSelect => self.dep_selector.data.len(),
            DisplayState::FeatureSelect => self.feature_selector.data.len(),
        };

        let mut offset = 0;

        if let DisplayState::FeatureSelect = self.state {
            if self.feature_selector.has_data() {
                let dep = self
                    .document
                    .get_dep(*self.dep_selector.get_selected().unwrap())
                    .unwrap();

                let feature_name = self.feature_selector.get_selected().unwrap();
                let data = dep.get_feature(feature_name);

                if !data.sub_features.is_empty() {
                    offset = 1;
                }
            }
        }

        let height = size().unwrap().1 as usize;

        let start = (current_selected - height as isize / 2 + 1)
            .min(max_range as isize - height as isize + 1 + offset as isize)
            .max(0) as usize;

        start..max_range.min(start + height - 1 - offset)
    }

    fn update_selected_data(&mut self) {
        match self.state {
            DisplayState::DepSelect => {
                self.dep_selector.data = self
                    .document
                    .get_deps_filtered_view(self.search_text.clone());
            }
            DisplayState::FeatureSelect => {
                let dep = self
                    .document
                    .get_dep(*self.dep_selector.get_selected().unwrap())
                    .unwrap();

                self.feature_selector.data =
                    dep.get_features_filtered_view(self.search_text.clone());
            }
        }
    }
}

enum DisplayState {
    DepSelect,
    FeatureSelect,
}

struct Selector<T> {
    selected: usize,

    data: Vec<T>,
}

impl<T> Selector<T> {
    fn shift(&mut self, shift: isize) {
        let mut selected_temp = self.selected as isize;

        selected_temp += self.data.len() as isize;
        selected_temp += shift;

        selected_temp %= self.data.len() as isize;

        self.selected = selected_temp as usize;
    }

    fn get_selected(&self) -> Option<&T> {
        self.data.get(self.selected)
    }

    fn has_data(&self) -> bool {
        !self.data.is_empty()
    }
}
