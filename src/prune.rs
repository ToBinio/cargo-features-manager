use crate::document::Document;
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::fs;

use console::{style, Term};
use std::io::Write;
use std::ops::Not;
use std::path::Path;

use std::process::{Command, Stdio};
use toml::Table;

pub fn prune(mut document: Document, is_dry_run: bool) -> anyhow::Result<()> {
    let mut term = Term::stdout();

    let ignored_features = get_ignored_features("./")?;

    for (index, name) in document.get_packages_names().iter().enumerate() {
        if document.is_workspace() {
            writeln!(term, "{}", name)?;
            prune_package(
                &mut document,
                is_dry_run,
                &mut term,
                index,
                2,
                &ignored_features,
            )?;
        } else {
            prune_package(
                &mut document,
                is_dry_run,
                &mut term,
                index,
                0,
                &ignored_features,
            )?;
        }
    }

    Ok(())
}

fn prune_package(
    document: &mut Document,
    is_dry_run: bool,
    term: &mut Term,
    package_id: usize,
    inset: usize,
    base_ignored: &HashMap<String, Vec<String>>,
) -> anyhow::Result<()> {
    let deps = document
        .get_deps(package_id)?
        .iter()
        .map(|dep| dep.get_name())
        .collect::<Vec<String>>();

    let ignored_features = get_ignored_features(
        document
            .get_package(package_id)
            .context("package not found")?
            .manifest_path
            .trim_end_matches("/Cargo.toml"),
    )?;

    for name in deps.iter() {
        let dependency = document.get_dep_mut(package_id, name)?;

        let enabled_features = dependency
            .features
            .iter()
            .filter(|(_name, data)| data.is_toggleable() && data.is_enabled())
            .filter(|(feature_name, _data)| {
                !ignored_features
                    .get(name)
                    .unwrap_or(&vec![])
                    .contains(feature_name)
            })
            .filter(|(feature_name, _data)| {
                !base_ignored
                    .get(name)
                    .unwrap_or(&vec![])
                    .contains(feature_name)
            })
            .map(|(name, _)| name)
            .cloned()
            .collect::<Vec<String>>();

        if enabled_features.is_empty() {
            continue;
        }

        term.clear_line()?;
        writeln!(term, "{:inset$}{} [0/0]", "", name)?;

        let mut to_be_disabled = vec![];

        for (id, feature) in enabled_features.iter().enumerate() {
            term.clear_line()?;
            writeln!(term, "{:inset$} └ {}", "", feature)?;

            document
                .get_dep_mut(package_id, name)?
                .disable_feature(feature)?;
            document.write_dep(package_id, name)?;

            if check()? {
                to_be_disabled.push(feature.to_string());
            }

            //reset to start
            for feature in &enabled_features {
                document
                    .get_dep_mut(package_id, name)?
                    .enable_feature(feature)?;
            }
            document.write_dep(package_id, name)?;

            term.move_cursor_up(2)?;
            term.clear_line()?;
            writeln!(
                term,
                "{:inset$}{} [{}/{}]",
                "",
                name,
                id + 1,
                enabled_features.len()
            )?;
        }

        let mut disabled_count = style(to_be_disabled.len());

        if to_be_disabled.is_empty().not() {
            disabled_count = disabled_count.red();
        }

        term.move_cursor_up(1)?;
        term.clear_line()?;
        writeln!(
            term,
            "{:inset$}{} [{}/{}]",
            "",
            name,
            disabled_count,
            enabled_features.len()
        )?;

        if is_dry_run {
            continue;
        }

        if to_be_disabled.is_empty().not() {
            for feature in to_be_disabled {
                document
                    .get_dep_mut(package_id, name)?
                    .disable_feature(&feature)?;
            }

            document.write_dep(package_id, name)?;
        }
    }
    Ok(())
}

fn check() -> anyhow::Result<bool> {
    if !build()? {
        return Ok(false);
    }

    if !test()? {
        return Ok(false);
    }

    Ok(true)
}

fn build() -> anyhow::Result<bool> {
    let mut child = Command::new("cargo")
        .arg("build")
        .arg("--all-targets")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let code = child.wait()?.code().ok_or(anyhow!("Could not build"))?;

    Ok(code == 0)
}

fn test() -> anyhow::Result<bool> {
    let mut child = Command::new("cargo")
        .arg("test")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let code = child.wait()?.code().ok_or(anyhow!("Could not test"))?;

    Ok(code == 0)
}

fn get_ignored_features<P: AsRef<Path>>(
    base_path: P,
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let result = fs::read_to_string(base_path.as_ref().join("Features.toml"));

    match result {
        Ok(file) => {
            let table = file.parse::<Table>()?;

            let mut map = HashMap::new();

            for (key, value) in table {
                map.insert(
                    key,
                    value
                        .as_array()
                        .ok_or(anyhow!("Invalid Features.toml format"))?
                        .iter()
                        .to_owned()
                        .filter_map(|value| value.as_str())
                        .map(|value| value.to_string())
                        .collect(),
                );
            }
            Ok(map)
        }
        Err(_) => Ok(HashMap::new()),
    }
}
