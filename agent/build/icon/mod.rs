mod gen;
mod meta;
pub(crate) mod render;
mod targets;

use crate::icon::gen::OutputGenerator;
use crate::icon::meta::{IconMetadata, IconTargetType};
use resvg::usvg;
use resvg::usvg::TreeParsing;
use thiserror::Error;

pub struct IconProcessor {
    icon: usvg::Tree,
    metadata: IconMetadata,
    output_generator: OutputGenerator,
}

impl IconProcessor {
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, IconProcessorError> {
        // Determine the resource directory
        let path = path.as_ref();
        let resource_dir = path.parent().ok_or(IconProcessorError::NoParentDir)?;

        // Load the metadata from the file
        let metadata = std::fs::read_to_string(path)?;
        let metadata = serde_json::from_str::<IconMetadata>(&metadata)
            .map_err(IconProcessorError::MetadataParse)?;

        // Load the icon based on the file specified in the metadata
        let icon_file = resource_dir.join(metadata.file.clone());
        // TODO: This should be done in the build script itself
        cargo_emit::rerun_if_changed!(icon_file.display());
        let icon_data = std::fs::read_to_string(icon_file)?;

        // Parse the icon
        let parse_options = usvg::Options {
            resources_dir: Some(resource_dir.to_path_buf()),
            ..Default::default()
        };

        let icon = usvg::Tree::from_str(&icon_data, &parse_options)
            .map_err(IconProcessorError::SvgParse)?;

        // Create the output generator
        let output_generator = OutputGenerator::new(resource_dir.join("build"));

        Ok(Self {
            metadata,
            icon,
            output_generator,
        })
    }

    pub fn metadata(&self) -> &IconMetadata {
        &self.metadata
    }

    pub fn process(&self, target: &str) -> Result<(), IconProcessorError> {
        let target = self
            .metadata
            .targets
            .iter()
            .find(|t| t.name == target)
            .ok_or_else(|| IconProcessorError::TargetNotFound(target.to_string()))?;

        // Check the processor to use
        match &target.target_type {
            IconTargetType::Png(png) => {
                targets::process_png_target(&self.icon, target, png, &self.output_generator)
            }
            IconTargetType::Ico(ico) => {
                targets::process_ico_target(&self.icon, target, ico, &self.output_generator)
            }
            IconTargetType::Other => {
                Err(IconProcessorError::UnsupportedTarget(target.name.clone()))
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum IconProcessorError {
    #[error("an I/O error occurred: {0}")]
    Io(#[from] std::io::Error),

    #[error("an error occurred while parsing the icon metadata: {0}")]
    MetadataParse(serde_json::Error),

    #[error("failed to parse SVG: {0}")]
    SvgParse(usvg::Error),

    #[error("the path specified as the metadata file has no parent directory")]
    NoParentDir,

    #[error("the target {0} was not found in the icon metadata")]
    TargetNotFound(String),

    #[error("the target {0} has a type not supported by this processor")]
    UnsupportedTarget(String),

    #[error("the pixmap dimensions are invalid: {width}x{height}")]
    InvalidPixmapDimensions { width: u32, height: u32 },

    #[error("an error occurred while encoding the PNG: {0}")]
    PngEncoding(#[from] png::EncodingError),
}
