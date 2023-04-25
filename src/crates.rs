use crates_index::Version;

pub struct Crate {
    version: Version,
}

impl Crate {
    pub fn new(version: Version) -> Crate {
        Crate { version }
    }

    pub fn get_name(&self) -> String {
        self.version.name().to_string()
    }

    pub fn get_unique_features(&self) -> Vec<String> {
        let mut features = vec![];

        for (name, sub) in self.version.features() {
            //skip if is is default
            if *name == "default" {
                continue;
            }

            features.push(name.clone());

            for name in sub {
                //skip if it is a dep or a feature of a dep
                if name.contains(':') || name.contains('/') {
                    continue;
                }

                features.push(name.clone());
            }
        }

        features.dedup();
        features.sort();

        features
    }
}
