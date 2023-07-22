use std::io::{stdout, Stdout, Write};
use std::ops::Range;

use crossterm::cursor::{Hide, MoveTo, RestorePosition, SavePosition, Show};
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor, StyledContent, Stylize};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{execute, queue};

use crate::document::Document;

pub struct Display {
    stdout: Stdout,

    document: Document,

    dep_selector: Selector<(usize, Vec<usize>)>,
    feature_selector: Selector<(String, Vec<usize>)>,

    state: DisplayState,

    search_text: String,
}

impl Display {
    pub fn new(document: Document) -> anyhow::Result<Display> {
        Ok(Display {
            stdout: stdout(),

            dep_selector: Selector {
                selected: 0,
                data: document.get_deps_filtered_view(""),
            },

            feature_selector: Selector {
                selected: 0,
                data: vec![],
            },

            document,

            state: DisplayState::DepSelect,
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
            .get_dep(self.dep_selector.get_selected().unwrap().0)
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

            if let RunningState::Finished = self.input_event()? {
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
            let word_mark = &dep.1;
            let dep = self.document.get_dep(dep.0).unwrap();

            if index == self.dep_selector.selected {
                queue!(self.stdout, MoveTo(0, line_index), Print(">"))?;
            }

            queue!(self.stdout, MoveTo(2, line_index))?;

            let word: Vec<StyledContent<String>> = dep
                .get_name()
                .chars()
                .enumerate()
                .map(|(index, c)| {
                    match (dep.has_features(), word_mark.contains(&index)) {
                        (true, true) => c.to_string().red(),
                        (true, false) => c.to_string().white(),
                        (false, true) => c.to_string().dark_red(),
                        // SetForegroundColor(Color::from((100, 100, 100)))
                        (false, false) => c.to_string().grey(),
                    }
                })
                .collect();

            for char in word {
                queue!(self.stdout, Print(char))?;
            }

            queue!(self.stdout, ResetColor)?;

            index += 1;
            line_index += 1;
        }

        self.display_type_mode()?;

        Ok(())
    }

    fn display_features(&mut self) -> anyhow::Result<()> {
        let dep = self
            .document
            .get_dep(self.dep_selector.get_selected().unwrap().0)
            .unwrap();

        let feature_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = feature_range.start;

        queue!(
            self.stdout,
            Print(format!("{} {}", dep.get_name(), dep.get_version())),
            SavePosition
        )?;

        for feature in &self.feature_selector.data[self.get_max_range()] {
            let data = dep.get_feature(&feature.0);

            if data.is_default {
                queue!(self.stdout, SetForegroundColor(Color::Green))?;
            }

            queue!(self.stdout, MoveTo(2, line_index), Print("["))?;

            if data.is_enabled {
                queue!(self.stdout, MoveTo(3, line_index), Print("X"))?;
            }

            queue!(self.stdout, MoveTo(4, line_index), Print("]"))?;
            queue!(self.stdout, ResetColor)?;

            if !dep.get_currently_dependent_features(&feature.0).is_empty() {
                queue!(
                    self.stdout,
                    SetForegroundColor(Color::from((100, 100, 100)))
                )?;
            }

            queue!(self.stdout, MoveTo(6, line_index))?;

            let word: Vec<StyledContent<String>> = feature
                .0
                .chars()
                .enumerate()
                .map(|(index, c)| match feature.1.contains(&index) {
                    true => c.to_string().red(),
                    false => c.to_string().white(),
                })
                .collect();

            for char in word {
                queue!(self.stdout, Print(char))?;
            }

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

        self.display_type_mode()?;

        Ok(())
    }

    /// crossterm::SavePosition has be been set before calling this function
    fn display_type_mode(&mut self) -> anyhow::Result<()> {
        if !self.search_text.is_empty() {
            queue!(
                self.stdout,
                RestorePosition,
                Print(format!(" - {}", self.search_text)),
            )?;
        }

        Ok(())
    }

    fn input_event(&mut self) -> anyhow::Result<RunningState> {
        if let Event::Key(key_event) = read()? {
            if let KeyEventKind::Press = key_event.kind {
                match (key_event.code, &self.state) {
                    //movement
                    //up
                    (KeyCode::Up, DisplayState::DepSelect) => {
                        self.dep_selector.shift(-1);
                    }
                    (KeyCode::Up, DisplayState::FeatureSelect) => {
                        if self.feature_selector.has_data() {
                            self.feature_selector.shift(-1);
                        }
                    }
                    //down
                    (KeyCode::Down, DisplayState::DepSelect) => {
                        self.dep_selector.shift(1);
                    }
                    (KeyCode::Down, DisplayState::FeatureSelect) => {
                        if self.feature_selector.has_data() {
                            self.feature_selector.shift(1);
                        }
                    }

                    //selection
                    (KeyCode::Enter, DisplayState::DepSelect)
                    | (KeyCode::Right, DisplayState::DepSelect)
                    | (KeyCode::Char(' '), DisplayState::DepSelect) => {
                        if self.dep_selector.has_data() {
                            self.search_text = "".to_string();

                            if self
                                .document
                                .get_dep(self.dep_selector.get_selected().unwrap().0)?
                                .has_features()
                            {
                                self.selected_dep();

                                //needed to wrap
                                self.feature_selector.shift(0);
                            }
                        }
                    }
                    (KeyCode::Enter, DisplayState::FeatureSelect)
                    | (KeyCode::Right, DisplayState::FeatureSelect)
                    | (KeyCode::Char(' '), DisplayState::FeatureSelect) => {
                        if self.feature_selector.has_data() {
                            let dep = self
                                .document
                                .get_dep_mut(self.dep_selector.get_selected().unwrap().0);

                            dep.toggle_feature_usage(
                                &self.feature_selector.get_selected().unwrap().0,
                            );

                            self.document.write_dep(self.dep_selector.selected);
                        }
                    }

                    (KeyCode::Char(char), _) => {
                        if char == ' ' {
                            return Ok(RunningState::Running);
                        }

                        self.search_text += char.to_string().as_str();

                        self.update_selected_data();

                        match self.state {
                            DisplayState::DepSelect => self.dep_selector.shift(0),
                            DisplayState::FeatureSelect => self.feature_selector.shift(0),
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        let _ = self.search_text.pop();

                        self.update_selected_data();
                    }

                    //back
                    (KeyCode::Esc, _) | (KeyCode::Left, _) => {
                        return Ok(self.move_back());
                    }

                    _ => {}
                }
            }
        }

        Ok(RunningState::Running)
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
                    .get_dep(self.dep_selector.get_selected().unwrap().0)
                    .unwrap();

                let feature_name = self.feature_selector.get_selected().unwrap();
                let data = dep.get_feature(&feature_name.0);

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
                self.dep_selector.data = self.document.get_deps_filtered_view(&self.search_text);
            }
            DisplayState::FeatureSelect => {
                let dep = self
                    .document
                    .get_dep(self.dep_selector.get_selected().unwrap().0)
                    .unwrap();

                self.feature_selector.data =
                    dep.get_features_filtered_view(self.search_text.clone());
            }
        }
    }

    fn move_back(&mut self) -> RunningState {
        match self.state {
            DisplayState::DepSelect => RunningState::Finished,
            DisplayState::FeatureSelect => {
                self.search_text = "".to_string();

                self.state = DisplayState::DepSelect;

                self.update_selected_data();
                RunningState::Running
            }
        }
    }
}

enum RunningState {
    Running,
    Finished,
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
        if !self.has_data() {
            self.selected = 0;
            return;
        }

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
