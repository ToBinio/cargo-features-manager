use crate::dependencies::dependency::DependencyType;
use anyhow::{anyhow, bail, Context};
use cargo_metadata::{DependencyKind, Target};
use cargo_platform::Platform;
use clap::builder::Str;
use itertools::Itertools;
use std::ops::Index;
use std::str::FromStr;

pub mod dependency;

pub fn get_path(kind: &DependencyType, target: &Option<Platform>) -> String {
    let path = match kind {
        DependencyType::Normal => "dependencies",
        DependencyType::Development => "dev-dependencies",
        DependencyType::Build => "build-dependencies",
        DependencyType::Workspace => "workspace.dependencies",
        DependencyType::Unknown => "dependencies",
    };

    if let Some(target) = target {
        return match target {
            Platform::Name(name) => format!("target.{}.{}", name, path),
            Platform::Cfg(cfg) => format!("target.'cfg({})'.{}", cfg, path),
        };
    }

    path.to_string()
}

pub fn get_mut_item_from_doc<'a>(
    path: &str,
    document: &'a mut toml_edit::Document,
) -> anyhow::Result<&'a mut toml_edit::Item> {
    let mut item = document.as_item_mut();

    let mut is_target = false;

    'outer: for key in path.split('.') {
        if is_target {
            is_target = false;

            let target = Platform::from_str(key.trim_start_matches("'").trim_end_matches("'"))?;

            let table = item
                .as_table_like_mut()
                .context(anyhow!("could not find - {} - no table", path))?;

            for (key, next_item) in table.iter_mut() {
                let platform =
                    Platform::from_str(key.trim_start_matches("'").trim_end_matches("'"))?;

                if platform.eq(&target) {
                    item = next_item;
                    continue 'outer;
                }
            }

            bail!("could not find - {} - no table", path)
        }

        item = item
            .get_mut(key)
            .context(anyhow!("could not find - {}", path))?;

        if key == "target" {
            is_target = true;
        }
    }

    Ok(item)
}

pub fn get_item_from_doc<'a>(
    path: &str,
    document: &'a toml_edit::Document,
) -> anyhow::Result<&'a toml_edit::Item> {
    let mut item = document.as_item();

    let mut is_target = false;

    'outer: for key in path.split('.') {
        if is_target {
            is_target = false;

            let target = Platform::from_str(key.trim_start_matches("'").trim_end_matches("'"))?;

            let table = item
                .as_table()
                .context(anyhow!("could not find - {} - no table", path))?;

            for (key, next_item) in table.iter() {
                let platform =
                    Platform::from_str(key.trim_start_matches("'").trim_end_matches("'"))?;

                if platform.eq(&target) {
                    item = next_item;
                    continue 'outer;
                }
            }

            bail!("could not find - {} - no table", path)
        }

        item = item
            .get(key)
            .context(anyhow!("could not find - {}", path))?;

        if key == "target" {
            is_target = true;
        }
    }

    Ok(item)
}
