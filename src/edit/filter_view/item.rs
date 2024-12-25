use crate::project::dependency::Dependency;
use crate::project::package::Package;
use crate::edit::search::highlight_search;
use console::style;

pub struct FilterViewItem {
    name: String,
    display_name: String,
}

impl FilterViewItem {
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

        if let Some(rename) = &dep.rename {
            display_name.push_str(&style(format!(" ({})", rename)).color256(8).to_string());
        }

        if let Some(comment) = &dep.comment {
            display_name.push_str(&style(format!(" ({})", comment)).color256(8).to_string());
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
