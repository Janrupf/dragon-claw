use serde::{Deserialize, Deserializer};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconMetadata {
    /// The path to the icon file
    pub file: PathBuf,

    /// The available targets for this icon
    pub targets: Vec<IconTarget>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconTarget {
    /// The name of the target
    pub name: String,

    /// The type of the target
    #[serde(flatten)]
    pub target_type: IconTargetType,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "options", rename_all = "camelCase")]
pub enum IconTargetType {
    /// PNG image
    Png(PngTarget),

    /// Windows icon
    Ico(IcoTarget),

    /// Other target types not supported by this processor
    #[serde(other, deserialize_with = "deserialize_other_target")]
    Other,
}

fn deserialize_other_target<'de, D>(_deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PngTarget {
    /// The width of the png
    pub width: u32,

    /// The height of the png
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcoTarget {
    /// The sizes to include in the ico file
    pub sizes: Vec<u32>,
}
