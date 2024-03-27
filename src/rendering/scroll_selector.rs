use crate::dependencies::dependency::{Dependency, DependencyType};
use anyhow::Context;

use console::{style, Emoji};

pub struct ScrollSelector<T> {
    pub selected_index: usize,
    pub data: Vec<T>,
}

impl<T> ScrollSelector<T> {
    pub fn shift(&mut self, shift: isize) {
        if !self.has_data() {
            self.selected_index = 0;
            return;
        }

        let mut selected_temp = self.selected_index as isize;

        selected_temp += self.data.len() as isize;
        selected_temp += shift;

        selected_temp %= self.data.len() as isize;

        self.selected_index = selected_temp as usize;
    }

    pub fn get_selected(&self) -> anyhow::Result<&T> {
        self.data
            .get(self.selected_index)
            .context("nothing selected")
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }
}

pub struct DependencySelectorItem {
    name: String,
    display_name: String,
}

impl DependencySelectorItem {
    pub fn new(dep: &Dependency, highlighted_letters: Vec<usize>) -> Self {
        let mut display_name: String = dep
            .get_name()
            .chars()
            .enumerate()
            .map(
                |(index, c)| match (dep.has_features(), highlighted_letters.contains(&index)) {
                    (true, true) => style(c).red().to_string(),
                    (true, false) => c.to_string(),
                    //dark red
                    (false, true) => style(c).color256(1).to_string(),
                    //light gray
                    (false, false) => style(c).color256(8).to_string(),
                },
            )
            .collect();

        display_name = match dep.kind {
            DependencyType::Normal | DependencyType::Workspace => display_name,
            DependencyType::Development => format!(
                "{} {}",
                Emoji("🧪", &style("dev").color256(8).to_string()),
                display_name
            )
            .to_string(),
            DependencyType::Build => format!(
                "{} {}",
                Emoji("🛠️", &style("dev").color256(8).to_string()),
                display_name
            )
            .to_string(),
            DependencyType::Unknown => format!(
                "{} {}",
                Emoji("❔", &style("dev").color256(8).to_string()),
                display_name
            )
            .to_string(),
        };

        if dep.workspace {
            display_name = format!("{} {}", Emoji("🗃️", ""), display_name).to_string();
        }

        Self {
            name: dep.get_name(),
            display_name,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

pub struct FeatureSelectorItem {
    name: String,
    display_name: String,
}

impl FeatureSelectorItem {
    pub fn new(name: &str, highlighted_letters: Vec<usize>) -> Self {
        let display_name: String = name
            .chars()
            .enumerate()
            .map(|(index, c)| match highlighted_letters.contains(&index) {
                true => style(c).red().to_string(),
                false => c.to_string(),
            })
            .collect();

        Self {
            name: name.to_string(),
            display_name,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}
