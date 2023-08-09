use crate::icon::meta::IconTarget;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct OutputGenerator {
    build_dir: PathBuf,
}

impl OutputGenerator {
    pub fn new(build_dir: PathBuf) -> Self {
        Self { build_dir }
    }

    /// Creates the output directory for the given target
    pub fn create_target_output_dir(&self, target: &IconTarget) -> Result<PathBuf, std::io::Error> {
        let target_dir = self.build_dir.join(&target.name);
        std::fs::create_dir_all(&target_dir)?;
        Ok(target_dir)
    }

    /// Creates a file in the output directory for the given target
    pub fn create_target_output(
        &self,
        target: &IconTarget,
        child_path: impl AsRef<Path>,
    ) -> Result<File, std::io::Error> {
        let target_dir = self.create_target_output_dir(target)?;
        let output_path = target_dir.join(child_path);

        File::create(output_path)
    }
}
