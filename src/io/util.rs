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

#[cfg(test)]
mod test {
    use cargo_metadata::cargo_platform::{Cfg, CfgExpr, Ident, Platform};
    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::{
        io::util::{get_dependecy_item_from_doc, get_dependency_path, toml_document_from_path},
        project::dependency::DependencyType,
    };

    fn simple_test_toml() -> NamedTempFile {
        let mut tmpfile = NamedTempFile::new().unwrap();
        write!(
            tmpfile,
            r#"[package]
name = "Test"
version = "0.0.1"
edition = "2024"

[dependencies]
clap_complete = "4.5.46"
console = {{ version = "0.16.1", features = ["std"], default-features = false }}
ctrlc = "3.4.5"

[dev-dependencies]
color-eyre = "0.6.3"
cargo_metadata = "0.23.0"
clap = {{ version = "4.5.31", features = ["derive"] }}
"#
        )
        .unwrap();

        tmpfile
    }

    #[test]
    fn toml_document_from_path_works() {
        let tmpfile = simple_test_toml();
        let doc = toml_document_from_path(tmpfile.path()).unwrap();
        assert!(doc.contains_table("package"));
    }

    #[test]
    fn get_dependecy_item_from_doc_works() {
        let tmpfile = simple_test_toml();
        let doc = toml_document_from_path(tmpfile.path()).unwrap();

        let item = get_dependecy_item_from_doc(&DependencyType::Development, &None, &doc).unwrap();
        assert!(item.as_table().unwrap().contains_key("color-eyre"));
    }

    #[test]
    fn get_dependency_path_works() {
        assert_eq!(
            get_dependency_path(
                &DependencyType::Normal,
                &Some(Platform::Name("x86_64".to_string())),
            ),
            "target.x86_64.dependencies"
        );

        assert_eq!(
            get_dependency_path(&DependencyType::Development, &None),
            "dev-dependencies"
        );

        assert_eq!(
            get_dependency_path(
                &DependencyType::Build,
                &Some(Platform::Cfg(CfgExpr::Value(Cfg::Name(Ident {
                    name: "x86_64".to_string(),
                    raw: false,
                })))),
            ),
            "target.'cfg(x86_64)'.build-dependencies"
        );

        assert_eq!(
            get_dependency_path(
                &DependencyType::Workspace,
                &Some(Platform::Name("x86_64".to_string())),
            ),
            "target.x86_64.workspace.dependencies"
        );
    }
}
