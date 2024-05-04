use crate::project::dependency::Dependency;
use color_eyre::eyre::{bail, eyre};

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    // path include the Cargo.toml
    pub manifest_path: String,
}

impl Package {
    pub fn get_deps(&self) -> &Vec<Dependency> {
        &self.dependencies
    }

    pub fn get_dep(&self, name: &str) -> color_eyre::Result<&Dependency> {
        let dep = self.dependencies.iter().find(|dep| dep.get_name().eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }

    pub fn get_dep_index(&self, name: &String) -> color_eyre::Result<usize> {
        Ok(self
            .dependencies
            .iter()
            .enumerate()
            .find(|(_, dep)| dep.get_name() == *name)
            .ok_or(eyre!("dependency \"{}\" could not be found", name))?
            .0)
    }

    pub fn get_dep_mut(&mut self, name: &str) -> color_eyre::Result<&mut Dependency> {
        let dep = self
            .dependencies
            .iter_mut()
            .find(|dep| dep.get_name().eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }
}
