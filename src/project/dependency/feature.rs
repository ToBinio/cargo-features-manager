use console::Emoji;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub struct FeatureData {
    pub sub_features: Vec<SubFeature>,
    pub is_default: bool,
    pub enabled_state: EnabledState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnabledState {
    Normal(bool),
    Workspace,
}

impl FeatureData {
    pub fn is_enabled(&self) -> bool {
        match self.enabled_state {
            EnabledState::Normal(is_enabled) => is_enabled,
            EnabledState::Workspace => true,
        }
    }

    pub fn is_toggleable(&self) -> bool {
        match self.enabled_state {
            EnabledState::Normal(_) => true,
            EnabledState::Workspace => false,
        }
    }

    pub fn has_dependency_features(&self) -> bool {
        self.sub_features.iter().any(|feature| {
            matches!(
                feature.kind,
                SubFeatureType::Dependency | SubFeatureType::DependencyFeature
            )
        })
    }
}

#[derive(Clone, Debug)]
pub struct SubFeature {
    pub name: String,
    pub kind: SubFeatureType,
}

/// https://doc.rust-lang.org/cargo/reference/features.html
#[derive(Clone, PartialEq, Debug)]
pub enum SubFeatureType {
    Normal,
    /// e.g. dep:gif
    Dependency,
    /// e.g. jpeg-decoder/rayon
    DependencyFeature,
}

impl Display for SubFeature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.kind == SubFeatureType::Dependency {
            f.write_str(&format!(
                "{}{}",
                Emoji("📦", "dep:"),
                self.name.trim_start_matches("dep:")
            ))?
        } else {
            f.write_str(&self.name)?
        }

        Ok(())
    }
}

impl From<&str> for SubFeatureType {
    fn from(s: &str) -> Self {
        if s.starts_with("dep:") {
            return SubFeatureType::Dependency;
        }

        if s.contains('/') {
            return SubFeatureType::DependencyFeature;
        }

        SubFeatureType::Normal
    }
}
