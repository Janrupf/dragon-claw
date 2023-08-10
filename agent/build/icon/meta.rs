use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconMetadata {
    /// The path to the icon file
    pub file: PathBuf,

    /// The available targets for this icon
    pub targets: Vec<IconTarget>,

    /// Variables to extract
    pub variables: HashMap<String, IconVariable>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IconVariable {
    /// A variable to be extract from a fill paint
    FillPaint(FillPaintVariable),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillPaintVariable {
    /// Id of the element to extract the fill paint from
    pub from: String,
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

    /// The placement of the png
    #[serde(default)]
    pub placement: PngTargetPlacement,

    /// Additional draw steps to perform
    #[serde(default)]
    pub draw_steps: DrawSteps,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PngTargetPlacement {
    /// The x coordinate of the png
    pub x: i32,

    /// The y coordinate of the png
    pub y: i32,

    /// The width of the png
    pub width: Option<u32>,

    /// The height of the png
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcoTarget {
    /// The sizes to include in the ico file
    pub sizes: Vec<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSteps {
    /// Steps to draw before drawing the icon
    #[serde(default)]
    pub before: Vec<DrawStep>,

    /// Steps to draw after drawing the icon
    #[serde(default)]
    pub after: Vec<DrawStep>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DrawStep {
    /// Draw a rectangle
    Rect(DrawRectStep),
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrawRectStep {
    /// The x coordinate of the rectangle
    pub x: u32,

    /// The y coordinate of the rectangle
    pub y: u32,

    /// The width of the rectangle
    pub width: u32,

    /// The height of the rectangle
    pub height: u32,

    /// The fill of the rectangle
    pub fill: InputUsage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InputUsage {
    /// Use a variable as input
    Variable { from: String },
}
