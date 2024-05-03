use crate::dependencies::dependency::EnabledState;

use color_eyre::eyre::{Context, ContextCompat};
use color_eyre::Result;
use console::{style, Emoji, Key, Term};
use std::io::Write;
use std::ops::{Not, Range};

use crate::document::Document;

use crate::rendering::scroll_selector::ScrollSelector;

pub struct Display {
    term: Term,

    document: Document,

    package_selector: ScrollSelector,
    dep_selector: ScrollSelector,
    feature_selector: ScrollSelector,

    state: DisplayState,

    search_text: String,
}

impl Display {
    pub fn new(document: Document) -> Result<Display> {
        Ok(Display {
            term: Term::buffered_stdout(),
            package_selector: ScrollSelector {
                selected_index: 0,
                data: document.get_package_names_filtered_view("")?,
            },
            dep_selector: ScrollSelector {
                selected_index: 0,
                data: document.get_deps_filtered_view(
                    &document.get_package_id(0).context("no package found")?.name,
                    "",
                )?,
            },
            feature_selector: ScrollSelector {
                selected_index: 0,
                data: vec![],
            },
            state: if document.is_workspace() {
                DisplayState::Package
            } else {
                DisplayState::Dep
            },
            search_text: "".to_string(),
            document,
        })
    }

    fn select_selected_package(&mut self) -> Result<()> {
        self.state = DisplayState::Dep;

        // update selector
        self.dep_selector.data = self
            .document
            .get_deps_filtered_view(self.package_selector.get_selected()?.name(), "")?;

        Ok(())
    }

    pub fn set_selected_dep(&mut self, dep_name: String) -> Result<()> {
        match self
            .document
            .get_dep_index(self.package_selector.get_selected()?.name(), &dep_name)
        {
            Ok(index) => {
                self.dep_selector.selected_index = index;

                self.select_selected_dep()?;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn select_selected_dep(&mut self) -> Result<()> {
        self.state = DisplayState::Feature;

        let dep = self.document.get_dep(
            self.package_selector.get_selected()?.name(),
            self.dep_selector.get_selected()?.name(),
        )?;

        // update selector
        self.feature_selector.data = dep.get_features_filtered_view(&self.search_text);

        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        //setup
        self.term.hide_cursor()?;

        for _ in 1..self.term.size().0 {
            writeln!(self.term)?;
        }

        self.term.move_cursor_to(0, 0)?;
        self.term.flush()?;

        loop {
            match self.state {
                DisplayState::Dep => self.display_deps()?,
                DisplayState::Feature => self.display_features()?,
                DisplayState::Package => self.display_packages()?,
            }

            self.term.flush()?;

            //clear previous screen
            self.term.clear_last_lines(self.term.size().0 as usize)?;
            if let RunningState::Finished = self.input_event()? {
                break;
            }
        }

        self.term.show_cursor()?;
        self.term.flush()?;

        Ok(())
    }

    fn display_packages(&mut self) -> Result<()> {
        write!(self.term, "Packages")?;
        self.display_search_header()?;

        let dep_range = self.get_max_range()?;

        let mut line_index = 1;
        let mut index = dep_range.start;

        for selected in &self.package_selector.data[dep_range] {
            if index == self.package_selector.selected_index {
                self.term.move_cursor_to(0, line_index)?;
                write!(self.term, ">")?;
            }

            self.term.move_cursor_to(2, line_index)?;
            write!(self.term, "{}", selected.display_name())?;

            index += 1;
            line_index += 1;
        }

        Ok(())
    }

    fn display_deps(&mut self) -> Result<()> {
        write!(self.term, "Dependencies")?;
        self.display_search_header()?;

        let dep_range = self.get_max_range()?;

        let mut line_index = 1;
        let mut index = dep_range.start;

        for selector in &self.dep_selector.data[dep_range] {
            if index == self.dep_selector.selected_index {
                self.term.move_cursor_to(0, line_index)?;
                write!(self.term, ">")?;
            }

            self.term.move_cursor_to(2, line_index)?;

            write!(self.term, "{}", selector.display_name())?;

            index += 1;
            line_index += 1;
        }

        Ok(())
    }

    fn display_features(&mut self) -> Result<()> {
        let dep = self
            .document
            .get_dep(
                self.package_selector.get_selected()?.name(),
                self.dep_selector.get_selected()?.name(),
            )
            .context(format!(
                "couldn't find {}",
                self.dep_selector.get_selected()?.name()
            ))?;

        let feature_range = self.get_max_range()?;

        let mut line_index = 1;
        let mut index = feature_range.start;

        write!(self.term, "{} {}", dep.get_name(), dep.get_version())?;

        self.display_search_header()?;

        let dep = self
            .document
            .get_dep(
                self.package_selector.get_selected()?.name(),
                self.dep_selector.get_selected()?.name(),
            )
            .context(format!(
                "could not find {}",
                self.dep_selector.get_selected()?.name()
            ))?;

        for feature in &self.feature_selector.data[self.get_max_range()?] {
            let data = dep
                .get_feature(feature.name())
                .context(format!("couldn't find {}", feature.name()))?;

            self.term.move_cursor_to(2, line_index)?;

            let marker = match data.enabled_state {
                EnabledState::Normal(is_enabled) => {
                    if is_enabled {
                        "[X]".to_string()
                    } else {
                        "[ ]".to_string()
                    }
                }
                EnabledState::Workspace => format!("{}", Emoji("🗃️", "W")),
            };

            if data.is_default {
                write!(self.term, "{}", style(marker).green())?;
            } else {
                write!(self.term, "{}", marker)?;
            }

            let mut feature_name = style(feature.display_name());

            if !dep
                .get_currently_dependent_features(feature.name())
                .is_empty()
                || data.enabled_state == EnabledState::Workspace
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

                    self.term.move_cursor_to(8, line_index)?;

                    for sub in sub_features {
                        write!(self.term, "{} ", sub)?;
                    }
                }
            }

            line_index += 1;
            index += 1;
        }

        Ok(())
    }

    fn display_search_header(&mut self) -> Result<()> {
        if !self.search_text.is_empty() {
            write!(self.term, " - {}", self.search_text)?;
        }

        Ok(())
    }

    fn input_event(&mut self) -> Result<RunningState> {
        match (self.term.read_key()?, &self.state) {
            //movement
            //up
            (Key::ArrowUp, DisplayState::Package) => {
                self.package_selector.shift(-1);
            }
            (Key::ArrowUp, DisplayState::Dep) => {
                self.dep_selector.shift(-1);
            }
            (Key::ArrowUp, DisplayState::Feature) => {
                if self.feature_selector.has_data() {
                    self.feature_selector.shift(-1);
                }
            }
            //down
            (Key::ArrowDown, DisplayState::Package) => {
                self.package_selector.shift(1);
            }
            (Key::ArrowDown, DisplayState::Dep) => {
                self.dep_selector.shift(1);
            }
            (Key::ArrowDown, DisplayState::Feature) => {
                if self.feature_selector.has_data() {
                    self.feature_selector.shift(1);
                }
            }

            //selection
            (Key::Enter, DisplayState::Package)
            | (Key::ArrowRight, DisplayState::Package)
            | (Key::Char(' '), DisplayState::Package) => {
                if self.package_selector.has_data() {
                    let name = self.package_selector.get_selected()?.name();

                    if !self
                        .document
                        .get_package(name)
                        .context(format!("package not found - {}", name))?
                        .dependencies
                        .is_empty()
                    {
                        self.search_text = "".to_string();

                        self.select_selected_package()?;

                        //needed to wrap
                        self.dep_selector.shift(0);
                    }
                }
            }
            (Key::Enter, DisplayState::Dep)
            | (Key::ArrowRight, DisplayState::Dep)
            | (Key::Char(' '), DisplayState::Dep) => {
                if self.dep_selector.has_data()
                    && self
                        .document
                        .get_dep(
                            self.package_selector.get_selected()?.name(),
                            self.dep_selector.get_selected()?.name(),
                        )?
                        .has_features()
                {
                    self.search_text = "".to_string();

                    self.select_selected_dep()?;

                    //needed to wrap
                    self.feature_selector.shift(0);
                }
            }
            (Key::Enter, DisplayState::Feature)
            | (Key::ArrowRight, DisplayState::Feature)
            | (Key::Char(' '), DisplayState::Feature) => {
                if self.feature_selector.has_data() {
                    let dep_name = self.dep_selector.get_selected()?.name();

                    let dep = self
                        .document
                        .get_dep_mut(self.package_selector.get_selected()?.name(), dep_name)?;

                    dep.toggle_feature(self.feature_selector.get_selected()?.name())?;

                    self.document
                        .write_dep(self.package_selector.get_selected()?.name(), dep_name)?;
                }
            }

            //search
            (Key::Char(char), _) => {
                if char == ' ' {
                    return Ok(RunningState::Running);
                }

                self.search_text += char.to_string().as_str();

                self.update_selected_data()?;

                match self.state {
                    DisplayState::Dep => self.dep_selector.shift(0),
                    DisplayState::Feature => self.feature_selector.shift(0),
                    DisplayState::Package => self.package_selector.shift(0),
                }
            }
            (Key::Backspace, _) => {
                let _ = self.search_text.pop();

                self.update_selected_data()?;
            }

            //back
            (Key::Escape, _) | (Key::ArrowLeft, _) => {
                return self.move_back();
            }

            _ => {}
        }

        Ok(RunningState::Running)
    }

    fn get_max_range(&self) -> Result<Range<usize>> {
        let current_selected = match self.state {
            DisplayState::Dep => self.dep_selector.selected_index,
            DisplayState::Feature => self.feature_selector.selected_index,
            DisplayState::Package => self.package_selector.selected_index,
        } as isize;

        let max_range = match self.state {
            DisplayState::Dep => self.dep_selector.data.len(),
            DisplayState::Feature => self.feature_selector.data.len(),
            DisplayState::Package => self.package_selector.data.len(),
        };

        let mut offset = 0;

        if let DisplayState::Feature = self.state {
            if self.feature_selector.has_data() {
                let dep = self.document.get_dep(
                    self.package_selector.get_selected()?.name(),
                    self.dep_selector.get_selected()?.name(),
                )?;

                let feature = self.feature_selector.get_selected()?;
                let data = dep
                    .get_feature(feature.name())
                    .context(format!("coundt find {}", feature.name()))?;

                if !data.sub_features.is_empty() {
                    offset = 1;
                }
            }
        }

        let height = self.term.size().0 as usize;

        let start = (current_selected - height as isize / 2 + 1)
            .min(max_range as isize - height as isize + 1 + offset as isize)
            .max(0) as usize;

        Ok(start..max_range.min(start + height - 1 - offset))
    }

    fn update_selected_data(&mut self) -> Result<()> {
        match self.state {
            DisplayState::Package => {
                self.package_selector.data = self
                    .document
                    .get_package_names_filtered_view(&self.search_text)?;
            }
            DisplayState::Dep => {
                self.dep_selector.data = self.document.get_deps_filtered_view(
                    self.package_selector.get_selected()?.name(),
                    &self.search_text,
                )?;
            }
            DisplayState::Feature => {
                let dep = self.document.get_dep(
                    self.package_selector.get_selected()?.name(),
                    self.dep_selector.get_selected()?.name(),
                )?;

                self.feature_selector.data = dep.get_features_filtered_view(&self.search_text);
            }
        }

        Ok(())
    }

    fn move_back(&mut self) -> Result<RunningState> {
        match self.state {
            DisplayState::Package => Ok(RunningState::Finished),
            DisplayState::Dep => {
                if !self.document.is_workspace() {
                    return Ok(RunningState::Finished);
                }

                self.search_text = "".to_string();

                self.state = DisplayState::Package;

                self.update_selected_data()?;
                Ok(RunningState::Running)
            }
            DisplayState::Feature => {
                self.search_text = "".to_string();

                self.state = DisplayState::Dep;

                self.update_selected_data()?;
                Ok(RunningState::Running)
            }
        }
    }
}

enum RunningState {
    Running,
    Finished,
}

enum DisplayState {
    Package,
    Dep,
    Feature,
}
