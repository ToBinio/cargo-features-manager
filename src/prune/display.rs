use crate::project::document::Document;
use crate::prune::{DependencyName, FeatureName, FeaturesMap};
use color_eyre::Result;
use console::{Term, style};
use itertools::Itertools;
use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::ops::Not;

type IsKnownFeature = bool;

pub struct Display {
    term: Term,

    package_inset: usize,
    dependency_inset: usize,

    feature_count: usize,
    checked_features_count: usize,
    is_workspace: bool,

    package_name: String,
    package_feature_count: usize,
    package_checked_features_count: usize,

    dependency_name: String,
    dependency_feature_count: usize,

    is_terminal: bool,
}

impl Display {
    pub fn new(features_to_test: &FeaturesMap, document: &Document) -> Self {
        let feature_count = features_to_test
            .values()
            .flat_map(|dependencies| dependencies.values())
            .flatten()
            .count();

        let package_inset = if features_to_test.len() == 1 { 0 } else { 2 };
        let dependency_inset = if features_to_test.len() == 1 { 2 } else { 4 };

        Self {
            feature_count,
            package_inset,
            dependency_inset,
            is_workspace: document.is_workspace(),
            package_name: "?".to_string(),
            package_feature_count: 0,
            package_checked_features_count: 0,
            dependency_name: "?".to_string(),
            term: Term::stdout(),
            checked_features_count: 0,
            dependency_feature_count: 0,
            is_terminal: std::io::stdout().is_terminal(),
        }
    }

    pub fn start(&self) -> Result<()> {
        writeln!(&self.term, "workspace [{}]", self.feature_count)?;
        if self.is_terminal {
            self.term.hide_cursor()?;
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<()> {
        if self.is_terminal {
            self.term.show_cursor()?;
        }
        Ok(())
    }

    pub fn display_known_features_notice(&mut self) -> Result<()> {
        if self.is_terminal {
            self.term.clear_line()?;
        }
        writeln!(self.term)?;
        writeln!(
            self.term,
            "Some features that do not affect compilation but can limit functionally where found. For more information refer to https://github.com/ToBinio/cargo-features-manager#prune"
        )?;
        Ok(())
    }

    pub fn next_package(
        &mut self,
        package_name: &str,
        package_features: &HashMap<DependencyName, Vec<FeatureName>>,
    ) -> Result<()> {
        if self.is_terminal.not() {
            return Ok(());
        }

        self.package_name = package_name.to_string();
        self.package_feature_count = package_features.values().flatten().count();
        self.package_checked_features_count = 0;

        if self.is_workspace {
            let package_inset = self.package_inset;

            self.term.clear_line()?;
            writeln!(self.term)?;
            writeln!(
                self.term,
                "{:package_inset$}{} [{}]",
                "", package_name, self.package_feature_count
            )?;
        }

        Ok(())
    }

    pub fn next_dependency(&mut self, name: &str, features: &[FeatureName]) {
        self.dependency_feature_count = features.len();
        self.dependency_name = name.to_string();
    }

    pub fn finish_dependency(
        &mut self,
        features: Vec<(&FeatureName, IsKnownFeature)>,
    ) -> Result<()> {
        let mut disabled_count = style(
            features
                .iter()
                .map(|(name, known)| {
                    if *known {
                        style(name).color256(7).to_string()
                    } else {
                        style(format!("-{}", name)).red().to_string()
                    }
                })
                .join(","),
        );

        if features.is_empty() {
            disabled_count = style("0".to_string());
        }

        let dependency_inset = self.dependency_inset;

        if self.is_terminal {
            self.term.clear_line()?;
        }
        writeln!(
            self.term,
            "{:dependency_inset$}{} [{}/{}]",
            "", self.dependency_name, disabled_count, self.dependency_feature_count
        )?;

        Ok(())
    }

    pub fn next_feature(&mut self, id: usize, feature_name: &FeatureName) -> Result<()> {
        if self.is_terminal.not() {
            return Ok(());
        }

        let dependency_inset = self.dependency_inset;

        self.term.clear_line()?;
        writeln!(
            self.term,
            "{:dependency_inset$}{} [{}/{}]",
            "", self.dependency_name, id, self.dependency_feature_count,
        )?;
        self.term.clear_line()?;
        writeln!(self.term, "{:dependency_inset$} └ {}", "", feature_name)?;
        self.term.move_cursor_up(2)?;

        self.display_progress_bar()?;

        Ok(())
    }

    pub fn finish_feature(&mut self) -> Result<()> {
        self.checked_features_count += 1;
        self.package_checked_features_count += 1;

        self.display_progress_bar()?;

        Ok(())
    }

    fn display_progress_bar(&mut self) -> Result<()> {
        if self.is_terminal.not() {
            return Ok(());
        }

        self.term.move_cursor_down(2)?;
        self.term.clear_line()?;
        writeln!(self.term)?;
        self.term.clear_line()?;
        write!(
            self.term,
            "Workspace [{}/{}]",
            self.checked_features_count, self.feature_count,
        )?;

        if self.is_workspace {
            write!(
                self.term,
                " -> {} [{}/{}]",
                self.package_name, self.package_checked_features_count, self.package_feature_count
            )?;
        }

        writeln!(self.term)?;

        self.term.move_cursor_up(4)?;
        Ok(())
    }
}
