use color_eyre::eyre::{bail, ContextCompat, eyre};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use cargo_platform::Platform;

pub fn toml_document_from_path<P: AsRef<Path>>(
    dir_path: P,
) -> color_eyre::Result<toml_edit::DocumentMut> {
    let file_content = fs::read_to_string(&dir_path)
        .map_err(|_| eyre!("could not find Cargo.toml at {:?}", dir_path.as_ref()))?;

    Ok(file_content.parse()?)
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