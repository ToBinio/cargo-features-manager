use crate::document::Document;
use anyhow::anyhow;
use std::collections::HashMap;
use std::fs;

use console::{style, Term};
use std::io::Write;
use std::ops::Not;

use std::process::{Command, Stdio};
use toml::Table;

pub fn prune(mut document: Document, is_dry_run: bool) -> anyhow::Result<()> {
    let mut term = Term::stdout();

    let deps = document
        .get_deps()
        .iter()
        .map(|dep| dep.get_name())
        .collect::<Vec<String>>();

    let ignored_features = get_ignored_features()?;

    for name in deps.iter() {
        let dependency = document.get_dep_mut(&name)?;

        let enabled_features = dependency
            .features
            .iter()
            .filter(|(_name, data)| data.is_enabled)
            .filter(|(feature_name, _data)| {
                !ignored_features
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
        writeln!(term, "{} [0/0]", name)?;

        let mut to_be_disabled = vec![];

        for (id, feature) in enabled_features.iter().enumerate() {
            term.clear_line()?;
            writeln!(term, " └ {}", feature)?;

            document.get_dep_mut(&name)?.disable_feature(feature);
            document.write_dep_by_name(&name)?;

            if check()? {
                to_be_disabled.push(feature.to_string());
            }

            //reset to start
            for feature in &enabled_features {
                document.get_dep_mut(&name)?.enable_feature(feature);
            }
            document.write_dep_by_name(&name)?;

            term.move_cursor_up(2)?;
            term.clear_line()?;
            writeln!(term, "{} [{}/{}]", name, id + 1, enabled_features.len())?;
        }

        let mut disabled_count = style(to_be_disabled.len());

        if to_be_disabled.is_empty().not() {
            disabled_count = disabled_count.red();
        }

        term.move_cursor_up(1)?;
        term.clear_line()?;
        writeln!(
            term,
            "{} [{}/{}]",
            name,
            disabled_count,
            enabled_features.len()
        )?;

        if is_dry_run {
            continue;
        }

        if to_be_disabled.is_empty().not() {
            for feature in to_be_disabled {
                document.get_dep_mut(&name)?.disable_feature(&feature);
            }

            document.write_dep_by_name(&name)?;
        }
    }

    Ok(())
}

fn check() -> anyhow::Result<bool> {
    let mut child = Command::new("cargo")
        .arg("check")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let code = child.wait()?.code().ok_or(anyhow!("Could not check"))?;

    Ok(code == 0)
}

fn get_ignored_features() -> anyhow::Result<HashMap<String, Vec<String>>> {
    let result = fs::read_to_string("Features.toml");

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
