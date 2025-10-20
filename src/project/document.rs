use color_eyre::eyre::{ContextCompat, bail, eyre};
use std::path::PathBuf;

use color_eyre::Result;
use itertools::Itertools;

use crate::io::parsing::package::get_packages;
use crate::project::dependency::feature::EnabledState;
use crate::project::package::Package;

pub struct Document {
    packages: Vec<Package>,
    workspace_index: Option<usize>,
    root_path: PathBuf,
}

impl Document {
    pub fn new(path: impl Into<PathBuf>) -> Result<Document> {
        let (mut packages, workspace, root_path) = get_packages(path)?;

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
            root_path,
        };

        document.update_workspace_deps()?;

        Ok(document)
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
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
                    .find(|workspace_dep| {
                        workspace_dep.name == dep.name
                            || workspace_dep
                                .rename
                                .as_ref()
                                .is_some_and(|name| name == dep.name.as_str())
                    })
                    .ok_or(eyre!(
                        "could not find workspace dep - {:#?}",
                        dep.get_name()
                    ))?;

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

    pub fn get_package_by_index(&self, package_id: usize) -> Result<&Package> {
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

    pub fn get_workspace_package(&self) -> Option<Result<&Package>> {
        self.workspace_index
            .map(|workspace_index| self.get_package_by_index(workspace_index))
    }

    pub fn is_workspace(&self) -> bool {
        self.packages.len() > 1
    }
}
