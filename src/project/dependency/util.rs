use crate::project::dependency::DependencyType;
use cargo_platform::Platform;

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
