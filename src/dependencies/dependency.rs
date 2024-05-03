use crate::rendering::scroll_selector::SelectorItem;
use cargo_metadata::DependencyKind;
use cargo_platform::Platform;
use color_eyre::eyre::{eyre, ContextCompat};
use color_eyre::Result;
use console::{style, Emoji};
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub rename: Option<String>,
    pub comment: Option<String>,
    pub version: String,

    pub workspace: bool,
    pub kind: DependencyType,
    pub target: Option<Platform>,

    pub features: HashMap<String, FeatureData>,
}

impl Dependency {
    pub fn get_name(&self) -> String {
        let mut name = if let Some(target) = &self.target {
            format!("{}.{}", target, self.name)
        } else {
            self.name.to_string()
        };

        name = match self.kind {
            DependencyType::Normal | DependencyType::Workspace => name,
            DependencyType::Development => format!(
                "{} {}",
                Emoji("🧪", &style("dev").color256(8).to_string()),
                name
            )
            .to_string(),
            DependencyType::Build => format!(
                "{} {}",
                Emoji("🛠️", &style("build").color256(8).to_string()),
                name
            )
            .to_string(),
            DependencyType::Unknown => format!(
                "{} {}",
                Emoji("❔", &style("unknown").color256(8).to_string()),
                name
            )
            .to_string(),
        };

        if self.workspace {
            name = format!("{} {}", Emoji("🗃️", "W"), name).to_string();
        }

        name
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
    }

    pub fn get_features_filtered_view(&self, filter: &str) -> Vec<SelectorItem> {
        let features = self
            .features
            .iter()
            .filter(|feature| feature.0 != "default");

        if filter.is_empty() {
            features
                .sorted_by(|(name_a, data_a), (name_b, data_b)| {
                    if data_a.is_default && !data_b.is_default {
                        return Ordering::Less;
                    }

                    if data_b.is_default && !data_a.is_default {
                        return Ordering::Greater;
                    }

                    name_a.cmp(name_b)
                })
                .map(|(name, _)| SelectorItem::from_feature(name, vec![]))
                .collect()
        } else {
            let matcher = SkimMatcherV2::default();

            features
                .filter_map(|(name, _)| matcher.fuzzy(name, filter, true).map(|some| (name, some)))
                .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
                .map(|(name, fuzzy)| (name, fuzzy.1))
                .map(|(name, indexes)| SelectorItem::from_feature(name, indexes))
                .collect()
        }
    }

    pub fn get_feature(&self, feature_name: &str) -> Option<&FeatureData> {
        self.features.get(feature_name)
    }

    pub fn has_features(&self) -> bool {
        !self.features.is_empty()
    }

    pub fn can_use_default(&self) -> bool {
        if self.workspace {
            return false;
        }

        for data in self.features.values() {
            if data.is_default && !data.is_enabled() {
                return false;
            }
        }

        true
    }

    pub fn get_features_to_enable(&self) -> Vec<String> {
        let can_use_default = self.can_use_default();

        self.features
            .iter()
            .filter(|(_, data)| data.is_enabled())
            .filter(|(_, data)| !can_use_default || !data.is_default)
            .filter(|(_, data)| data.enabled_state != EnabledState::Workspace)
            .map(|(name, _)| name.clone())
            .filter(|name| name != "default")
            .filter(|name| self.get_currently_dependent_features(name).is_empty())
            .sorted()
            .collect()
    }

    pub fn toggle_feature(&mut self, feature_name: &str) -> Result<()> {
        let data = self
            .features
            .get(feature_name)
            .context(format!("could not find {}", feature_name))?;

        if let EnabledState::Normal(is_enabled) = data.enabled_state {
            if is_enabled {
                self.disable_feature(feature_name)?;
            } else {
                self.enable_feature(feature_name)?;
            }
        }

        Ok(())
    }

    pub fn set_feature_to_workspace(&mut self, feature_name: &str) -> Result<()> {
        let data = self.features.get_mut(feature_name).ok_or(eyre!(
            "couldnt find feature {} trying to set as workspace feature for {}",
            feature_name,
            self.name
        ))?;

        data.enabled_state = EnabledState::Workspace;

        Ok(())
    }

    pub fn enable_feature(&mut self, feature_name: &str) -> Result<()> {
        let data = self.features.get_mut(feature_name).ok_or(eyre!(
            "couldnt find feature {} trying to enable for {}",
            feature_name,
            self.name
        ))?;

        if data.is_enabled() {
            //early return to prevent loop
            return Ok(());
        }

        data.enabled_state = EnabledState::Normal(true);

        //enable sub features
        let sub_features = data
            .sub_features
            .iter()
            .filter(|sub_feature| sub_feature.kind == FeatureType::Normal)
            .map(|sub_feature| sub_feature.name.to_string())
            .collect_vec();

        for sub_feature_name in sub_features {
            self.enable_feature(&sub_feature_name)?;
        }

        Ok(())
    }

    pub fn disable_feature(&mut self, feature_name: &str) -> Result<()> {
        let data = self
            .features
            .get_mut(feature_name)
            .context(format!("could not find {}", feature_name))?;

        if !data.is_enabled() {
            //early return to prevent loop
            return Ok(());
        }

        data.enabled_state = EnabledState::Normal(false);

        for name in self.get_dependent_features(feature_name) {
            self.disable_feature(&name)?
        }

        Ok(())
    }

    /// returns all features which require the feature to be enabled
    fn get_dependent_features(&self, feature_name: &str) -> Vec<String> {
        let mut dep_features = vec![];

        for (name, data) in &self.features {
            if data
                .sub_features
                .iter()
                .any(|sub_feature| sub_feature.name == *feature_name)
            {
                dep_features.push(name.to_string())
            }
        }

        dep_features
    }

    /// returns all features which are currently enabled and require the feature to be enabled
    pub fn get_currently_dependent_features(&self, feature_name: &str) -> Vec<String> {
        self.get_dependent_features(feature_name)
            .iter()
            .filter_map(|name| self.features.get(name).map(|feature| (name, feature)))
            .filter(|(_, feature)| feature.is_enabled())
            .map(|(name, _)| name.to_string())
            .collect()
    }
}

#[derive(Debug)]
pub enum DependencyType {
    Normal,
    Development,
    Build,
    Workspace,
    Unknown,
}

impl From<DependencyKind> for DependencyType {
    fn from(value: DependencyKind) -> Self {
        match value {
            DependencyKind::Normal => DependencyType::Normal,
            DependencyKind::Development => DependencyType::Development,
            DependencyKind::Build => DependencyType::Build,
            DependencyKind::Unknown => DependencyType::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FeatureData {
    pub sub_features: Vec<SubFeature>,
    pub is_default: bool,
    pub enabled_state: EnabledState,
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnabledState {
    Normal(bool),
    Workspace,
}

#[derive(Clone, Debug)]
pub struct SubFeature {
    pub name: String,
    pub kind: FeatureType,
}

#[derive(Clone, PartialEq, Debug)]
pub enum FeatureType {
    Normal,
    Dependency,
    DependencyFeature,
}

impl Display for SubFeature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.kind == FeatureType::Dependency {
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

impl From<&str> for FeatureType {
    fn from(s: &str) -> Self {
        if s.starts_with("dep:") {
            return FeatureType::Dependency;
        }

        if s.contains('/') {
            return FeatureType::DependencyFeature;
        }

        FeatureType::Normal
    }
}
