use console::{style, Key, Term};
use std::io::{stdout, Stdout, Write};
use std::ops::{Not, Range};
use std::ptr::write;

use crate::document::Document;
use crate::scroll_selector::{DependencySelectorItem, FeatureSelectorItem, ScrollSelector};

pub struct Display {
    term: Term,

    document: Document,

    dep_selector: ScrollSelector<DependencySelectorItem>,
    feature_selector: ScrollSelector<FeatureSelectorItem>,

    state: DisplayState,

    search_text: String,
}

impl Display {
    pub fn new(document: Document) -> anyhow::Result<Display> {
        Ok(Display {
            term: Term::buffered_stdout(),

            dep_selector: ScrollSelector {
                selected_index: 0,
                data: document.get_deps_filtered_view(""),
            },

            feature_selector: ScrollSelector {
                selected_index: 0,
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
                self.dep_selector.selected_index = index;

                self.select_selected_dep();
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn select_selected_dep(&mut self) {
        self.state = DisplayState::FeatureSelect;

        let dep = self
            .document
            .get_dep(self.dep_selector.get_selected().unwrap().name())
            .unwrap();

        // update selector
        self.feature_selector.data = dep.get_features_filtered_view(&self.search_text);
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        //setup
        self.term.hide_cursor()?;

        loop {
            //clear previous screen
            self.term.clear_last_lines(self.term.size().0 as usize)?;

            match self.state {
                DisplayState::DepSelect => self.display_deps()?,
                DisplayState::FeatureSelect => self.display_features()?,
            }

            self.term.flush()?;

            if let RunningState::Finished = self.input_event()? {
                break;
            }
        }

        self.term.show_cursor()?;

        Ok(())
    }

    fn display_deps(&mut self) -> anyhow::Result<()> {
        write!(self.term, "Dependencies")?;
        self.display_search_header()?;

        let dep_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = dep_range.start;

        for selector in &self.dep_selector.data[dep_range] {
            let _dep = self.document.get_dep(selector.name()).unwrap();

            if index == self.dep_selector.selected_index {
                self.term.move_cursor_to(0, line_index)?;
                write!(self.term, ">")?;
            }

            self.term.move_cursor_to(2, line_index)?;
            //todo?
            write!(self.term, "{}", selector.display_name())?;

            index += 1;
            line_index += 1;
        }

        Ok(())
    }

    fn display_features(&mut self) -> anyhow::Result<()> {
        let dep = self
            .document
            .get_dep(self.dep_selector.get_selected().unwrap().name())
            .unwrap();

        let feature_range = self.get_max_range();

        let mut line_index = 1;
        let mut index = feature_range.start;

        write!(self.term, "{} {}", dep.get_name(), dep.get_version())?;

        self.display_search_header()?;

        //todo
        let dep = self
            .document
            .get_dep(self.dep_selector.get_selected().unwrap().name())
            .unwrap();

        for feature in &self.feature_selector.data[self.get_max_range()] {
            let data = dep.get_feature(feature.name());

            self.term.move_cursor_to(2, line_index)?;

            let marker = if data.is_enabled { "[X]" } else { "[ ]" };

            if data.is_default {
                write!(self.term, "{}", style(marker).green())?;
            } else {
                write!(self.term, "{}", marker)?;
            }

            let mut feature_name = style(feature.display_name());

            if !dep
                .get_currently_dependent_features(feature.name())
                .is_empty()
            {
                //gray
                feature_name = feature_name.color256(8);
            }

            self.term.move_cursor_right(1)?;
            write!(self.term, "{}", feature_name)?;

            if index == self.feature_selector.selected_index {
                self.term.move_cursor_to(0, line_index)?;
                write!(self.term, ">")?;

                let sub_features = &data.sub_features;

                if sub_features.is_empty().not() {
                    line_index += 1;

                    self.term.move_cursor_to(6, line_index)?;
                    write!(self.term, "└")?;

                    //todo print direct ?
                    let mut sub_features_str = "".to_string();

                    for sub_feature in sub_features {
                        sub_features_str += sub_feature;
                        sub_features_str += " ";
                    }

                    self.term.move_cursor_to(8, line_index)?;
                    write!(self.term, "{}", sub_features_str)?;
                }
            }

            line_index += 1;
            index += 1;
        }

        Ok(())
    }

    fn display_search_header(&mut self) -> anyhow::Result<()> {
        if !self.search_text.is_empty() {
            write!(self.term, " - {}", self.search_text)?;
        }

        Ok(())
    }

    fn input_event(&mut self) -> anyhow::Result<RunningState> {
        match (self.term.read_key()?, &self.state) {
            //movement
            //up
            (Key::ArrowUp, DisplayState::DepSelect) => {
                self.dep_selector.shift(-1);
            }
            (Key::ArrowUp, DisplayState::FeatureSelect) => {
                if self.feature_selector.has_data() {
                    self.feature_selector.shift(-1);
                }
            }
            //down
            (Key::ArrowDown, DisplayState::DepSelect) => {
                self.dep_selector.shift(1);
            }
            (Key::ArrowDown, DisplayState::FeatureSelect) => {
                if self.feature_selector.has_data() {
                    self.feature_selector.shift(1);
                }
            }

            //selection
            (Key::Enter, DisplayState::DepSelect)
            | (Key::ArrowRight, DisplayState::DepSelect)
            | (Key::Char(' '), DisplayState::DepSelect) => {
                if self.dep_selector.has_data() {
                    self.search_text = "".to_string();

                    if self
                        .document
                        .get_dep(self.dep_selector.get_selected().unwrap().name())?
                        .has_features()
                    {
                        self.select_selected_dep();

                        //needed to wrap
                        self.feature_selector.shift(0);
                    }
                }
            }
            (Key::Enter, DisplayState::FeatureSelect)
            | (Key::ArrowRight, DisplayState::FeatureSelect)
            | (Key::Char(' '), DisplayState::FeatureSelect) => {
                if self.feature_selector.has_data() {
                    let dep = self
                        .document
                        .get_dep_mut(self.dep_selector.get_selected().unwrap().name())?;

                    dep.toggle_feature_usage(self.feature_selector.get_selected().unwrap().name());

                    self.document.write_dep(self.dep_selector.selected_index);
                }
            }

            (Key::Char(char), _) => {
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
            (Key::Backspace, _) => {
                let _ = self.search_text.pop();

                self.update_selected_data();
            }

            //back
            (Key::Escape, _) | (Key::ArrowLeft, _) => {
                return Ok(self.move_back());
            }

            _ => {}
        }

        Ok(RunningState::Running)
    }

    fn get_max_range(&self) -> Range<usize> {
        let current_selected = match self.state {
            DisplayState::DepSelect => self.dep_selector.selected_index,
            DisplayState::FeatureSelect => self.feature_selector.selected_index,
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
                    .get_dep(self.dep_selector.get_selected().unwrap().name())
                    .unwrap();

                let feature_name = self.feature_selector.get_selected().unwrap();
                let data = dep.get_feature(feature_name.name());

                if !data.sub_features.is_empty() {
                    offset = 1;
                }
            }
        }

        let height = self.term.size().0 as usize;

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
                    .get_dep(self.dep_selector.get_selected().unwrap().name())
                    .unwrap();

                self.feature_selector.data = dep.get_features_filtered_view(&self.search_text);
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
