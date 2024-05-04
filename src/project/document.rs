use color_eyre::eyre::{bail, eyre, ContextCompat};

use color_eyre::Result;
use itertools::Itertools;

use crate::parsing::package::get_packages;
use crate::project::dependency::feature::EnabledState;
use crate::project::package::Package;

pub struct Document {
    packages: Vec<Package>,
    workspace_index: Option<usize>,
}

impl Document {
    pub fn new() -> Result<Document> {
        let (mut packages, workspace) = get_packages()?;

        if packages.len() == 1
            && packages
                .first()
                .context("no package found")?
                .dependencies
                .is_empty()
        {
            bail!("no dependencies were found")
        }

        let mut workspace_index = None;

        if let Some(workspace) = workspace {
            packages.push(workspace);

            workspace_index = Some(packages.len() - 1);
        }

        let mut document = Document {
            packages,
            workspace_index,
        };

        document.update_workspace_deps()?;

        Ok(document)
    }

    pub fn update_workspace_deps(&mut self) -> Result<()> {
        let Some(workspace_index) = self.workspace_index else {
            return Ok(());
        };

        for index in 0..self.packages.len() {
            if index == workspace_index {
                continue;
            };

            for dep_index in 0..self.packages[index].dependencies.len() {
                let dep = &self.packages[index].dependencies[dep_index];

                if !dep.workspace {
                    continue;
                }

                let workspace = &self.packages[workspace_index];
                let workspace_dep = workspace
                    .dependencies
                    .iter()
                    .find(|workspace_dep| workspace_dep.name == dep.name)
                    .ok_or(eyre!("could not find workspace dep - {}", dep.get_name()))?;

                let enabled_workspace_features = workspace_dep
                    .features
                    .iter()
                    .filter(|(_, data)| data.is_enabled())
                    .map(|(name, _)| name.to_string())
                    .collect_vec();

                let dep = &mut self.packages[index].dependencies[dep_index];

                let workspace_deps = dep
                    .features
                    .iter()
                    .filter(|(_, data)| data.enabled_state == EnabledState::Workspace)
                    .map(|(name, _)| name.to_string())
                    .collect_vec();

                for feature in workspace_deps {
                    dep.disable_feature(feature.as_str())?;
                }

                for name in enabled_workspace_features {
                    dep.enable_feature(&name)?;
                    dep.set_feature_to_workspace(&name)?;
                }
            }
        }

        Ok(())
    }

    pub fn get_packages(&self) -> &Vec<Package> {
        &self.packages
    }

    pub fn get_packages_names(&self) -> Vec<String> {
        self.packages
            .iter()
            .map(|package| package.name.to_string())
            .collect()
    }

    pub fn get_package_by_id(&self, package_id: usize) -> Result<&Package> {
        self.packages
            .get(package_id)
            .context(format!("no package for id {} found", package_id))
    }

    pub fn get_package(&self, package: &str) -> Result<&Package> {
        self.packages
            .iter()
            .find(|pkg| pkg.name == package)
            .context(format!("no package with name {} found", package))
    }

    pub fn get_package_mut(&mut self, package: &str) -> Result<&mut Package> {
        self.packages
            .iter_mut()
            .find(|pkg| pkg.name == package)
            .context(format!("no package with name {} found", package))
    }

    pub fn workspace_index(&self) -> Option<usize> {
        self.workspace_index
    }
    pub fn is_workspace(&self) -> bool {
        self.packages.len() > 1
    }
}
