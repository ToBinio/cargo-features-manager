use crate::io::util::{get_mut_item_from_doc, toml_document_from_path};
use crate::project::dependency::util::get_path;
use crate::project::document::Document;
use color_eyre::eyre::{ContextCompat, Error};
use std::fs;
use toml_edit::{Array, Formatted, InlineTable, Item, Value};

pub fn save_dependency(
    document: &mut Document,
    package_name: &str,
    dep_name: &str,
) -> color_eyre::Result<()> {
    let package = document.get_package_mut(package_name)?;
    let dependency = package.get_dep(dep_name)?;

    let features_to_enable = dependency.get_features_to_enable();

    let mut doc = toml_document_from_path(&package.manifest_path)?;
    let deps = get_mut_item_from_doc(&get_path(&dependency.kind, &dependency.target), &mut doc)?;

    let deps = deps.as_table_mut().context(format!(
        "could not parse dependencies as a table - {}",
        package.name
    ))?;

    let table = match deps
        .get_mut(dependency.rename.as_ref().unwrap_or(&dependency.name))
        .context("dependency not found")?
        .as_table_like_mut()
    {
        None => {
            deps.insert(
                &dependency.name,
                Item::Value(Value::InlineTable(InlineTable::new())),
            );

            deps.get_mut(&dependency.name)
                .context(format!(
                    "could not find {} in dependency",
                    dependency.get_name()
                ))?
                .as_table_like_mut()
                .context(format!(
                    "could not parse {} as a table",
                    dependency.get_name()
                ))?
        }
        Some(table) => table,
    };

    let has_custom_attributes = table
        .get_values()
        .iter()
        .map(|(name, _)| name.first().map(|key| key.to_string()).unwrap_or_default())
        .any(|name| !["features", "default-features", "version"].contains(&&*name));

    //check if entry has to be table or can just be string with version
    if dependency.can_use_default() && features_to_enable.is_empty() && !has_custom_attributes {
        deps.insert(
            &dependency.name,
            Item::Value(Value::String(Formatted::new(dependency.get_version()))),
        );
    } else {
        //version
        if !dependency.version.is_empty() && !table.contains_key("git") && !dependency.workspace {
            table.insert(
                "version",
                Item::Value(Value::String(Formatted::new(dependency.get_version()))),
            );
        }

        //features
        let mut features = Array::new();

        for name in features_to_enable {
            features.push(Value::String(Formatted::new(name)));
        }

        if features.is_empty() {
            table.remove("features");
        } else {
            table.insert("features", Item::Value(Value::Array(features)));
        }

        //default-feature
        if dependency.can_use_default() || dependency.workspace {
            table.remove("default-features");
        } else {
            table.insert(
                "default-features",
                Item::Value(Value::Boolean(Formatted::new(false))),
            );
        }
    }

    // update workspace deps
    if let Some(workspace_index) = document.workspace_index() {
        let workspace = document.get_package_by_id(workspace_index)?;

        if workspace.name == package_name {
            document.update_workspace_deps()?;
        }
    }

    //write updates
    let package = document.get_package(package_name)?;

    fs::write(&package.manifest_path, doc.to_string()).map_err(Error::from)
}
