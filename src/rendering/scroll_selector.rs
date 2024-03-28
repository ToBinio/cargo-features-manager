use crate::dependencies::dependency::{Dependency, DependencyType};
use anyhow::Context;

use crate::parsing::package::Package;
use crate::rendering::search::highlight_search;
use console::{style, Emoji};

pub struct ScrollSelector {
    pub selected_index: usize,
    pub data: Vec<SelectorItem>,
}

impl ScrollSelector {
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

    pub fn get_selected(&self) -> anyhow::Result<&SelectorItem> {
        self.data
            .get(self.selected_index)
            .context("nothing selected")
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }
}

pub struct SelectorItem {
    name: String,
    display_name: String,
}

impl SelectorItem {
    pub fn from_package(dep: &Package, highlighted_letters: Vec<usize>) -> Self {
        Self {
            name: dep.name.to_string(),
            display_name: highlight_search(
                &dep.name,
                &highlighted_letters,
                dep.dependencies.is_empty(),
            ),
        }
    }

    pub fn from_dependency(dep: &Dependency, highlighted_letters: Vec<usize>) -> Self {
        let mut display_name =
            highlight_search(&dep.get_name(), &highlighted_letters, !dep.has_features());

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
                Emoji("🛠️", &style("build").color256(8).to_string()),
                display_name
            )
            .to_string(),
            DependencyType::Unknown => format!(
                "{} {}",
                Emoji("❔", &style("unknown").color256(8).to_string()),
                display_name
            )
            .to_string(),
        };

        if dep.workspace {
            display_name = format!("{} {}", Emoji("🗃️", "W"), display_name).to_string();
        }

        Self {
            name: dep.get_name(),
            display_name,
        }
    }

    pub fn from_feature(name: &str, highlighted_letters: Vec<usize>) -> Self {
        Self {
            name: name.to_string(),
            display_name: highlight_search(name, &highlighted_letters, false),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}
