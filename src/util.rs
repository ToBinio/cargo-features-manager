use color_eyre::eyre::eyre;
use std::fs;
use std::path::Path;

pub fn toml_document_from_path<P: AsRef<Path>>(
    dir_path: P,
) -> color_eyre::Result<toml_edit::DocumentMut> {
    let file_content = fs::read_to_string(&dir_path)
        .map_err(|_| eyre!("could not find Cargo.toml at {:?}", dir_path.as_ref()))?;

    Ok(file_content.parse()?)
}
