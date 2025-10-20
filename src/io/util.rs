use cargo_metadata::cargo_platform::Platform;
use color_eyre::eyre::{ContextCompat, bail, eyre};
use std::fs;
use std::path::Path;
use std::str::FromStr;

use crate::project::dependency::DependencyType;

pub fn toml_document_from_path<P: AsRef<Path>>(
    dir_path: P,
) -> color_eyre::Result<toml_edit::DocumentMut> {
    let file_content = fs::read_to_string(&dir_path).map_err(|err| {
        eyre!(
            "could not find Cargo.toml at {:?} - {err}",
            dir_path.as_ref()
        )
    })?;

    Ok(file_content.parse()?)
}

pub fn get_mut_dependecy_item_from_doc<'a>(
    kind: &DependencyType,
    target: &Option<Platform>,
    document: &'a mut toml_edit::DocumentMut,
) -> color_eyre::Result<&'a mut toml_edit::Item> {
    let path = get_dependency_path(kind, target);
    get_mut_item_from_doc(&path, document)
}

pub fn get_mut_item_from_doc<'a>(
    path: &str,
    document: &'a mut toml_edit::DocumentMut,
) -> color_eyre::Result<&'a mut toml_edit::Item> {
    let mut item = document.as_item_mut();

    let mut is_target = false;

    'outer: for key in path.split('.') {
        if is_target {
            is_target = false;

            let target = Platform::from_str(key.trim_start_matches('\'').trim_end_matches('\''))?;

            let table = item
                .as_table_like_mut()
                .context(eyre!("could not find - {} - no table", path))?;

            for (key, next_item) in table.iter_mut() {
                let platform =
                    Platform::from_str(key.trim_start_matches('\'').trim_end_matches('\''))?;

                if platform.eq(&target) {
                    item = next_item;
                    continue 'outer;
                }
            }

            bail!("could not find - {} - no table", path)
        }

        item = item
            .get_mut(key)
            .context(eyre!("could not find - {}", path))?;

        if key == "target" {
            is_target = true;
        }
    }

    Ok(item)
}

pub fn get_dependecy_item_from_doc<'a>(
    kind: &DependencyType,
    target: &Option<Platform>,
    document: &'a toml_edit::DocumentMut,
) -> color_eyre::Result<&'a toml_edit::Item> {
    let path = get_dependency_path(kind, target);
    get_item_from_doc(&path, document)
}

pub fn get_item_from_doc<'a>(
    path: &str,
    document: &'a toml_edit::DocumentMut,
) -> color_eyre::Result<&'a toml_edit::Item> {
    let mut item = document.as_item();

    let mut is_target = false;

    'outer: for key in path.split('.') {
        if is_target {
            is_target = false;

            let target = Platform::from_str(key.trim_start_matches('\'').trim_end_matches('\''))?;

            let table = item
                .as_table()
                .context(eyre!("could not find - {} - no table", path))?;

            for (key, next_item) in table.iter() {
                let platform =
                    Platform::from_str(key.trim_start_matches('\'').trim_end_matches('\''))?;

                if platform.eq(&target) {
                    item = next_item;
                    continue 'outer;
                }
            }

            bail!("could not find - {} - no table", path)
        }

        item = item.get(key).context(eyre!("could not find - {}", path))?;

        if key == "target" {
            is_target = true;
        }
    }

    Ok(item)
}

fn get_dependency_path(kind: &DependencyType, target: &Option<Platform>) -> String {
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
