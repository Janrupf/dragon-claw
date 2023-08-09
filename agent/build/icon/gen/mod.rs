use crate::icon::meta::IconTarget;
use crate::icon::IconProcessorError;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::ops::Deref;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct OutputGenerator {
    target_dir: PathBuf,
    generated_outputs: HashMap<String, PathBuf>,
}

impl OutputGenerator {
    pub fn new(build_dir: &Path, target: &IconTarget) -> Self {
        Self {
            target_dir: build_dir.join(&target.name),
            generated_outputs: HashMap::new(),
        }
    }

    /// Creates a file in the output directory for the given target
    pub fn create_target_output(
        &mut self,
        name: impl Into<String>,
        child_path: impl AsRef<Path>,
    ) -> Result<File, IconProcessorError> {
        // Make sure there are no duplicated outputs
        let entry = match self.generated_outputs.entry(name.into()) {
            Entry::Occupied(v) => {
                return Err(IconProcessorError::DuplicatedOutput {
                    name: v.key().clone(),
                    path: v.get().clone(),
                })
            }
            Entry::Vacant(e) => e,
        };

        std::fs::create_dir_all(&self.target_dir)?;
        let output_path = self.target_dir.join(child_path);

        // Create the output file and then register the output
        let f = File::create(&output_path)?;
        entry.insert(output_path);

        Ok(f)
    }

    /// Finalizes the output generator and returns the outputs
    pub fn finalize(self) -> BuildOutputs {
        BuildOutputs {
            generated_outputs: self.generated_outputs,
        }
    }
}

#[derive(Debug)]
pub struct BuildOutputs {
    generated_outputs: HashMap<String, PathBuf>,
}

impl BuildOutputs {
    /// Gets the output path for the given name
    pub fn get_output(&self, name: impl AsRef<str>) -> Result<&Path, IconProcessorError> {
        let name = name.as_ref();

        self.generated_outputs
            .get(name)
            .map(|v| (*v).deref())
            .ok_or_else(|| IconProcessorError::OutputNotFound(name.to_string()))
    }
}
