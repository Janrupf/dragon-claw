use crate::icon::gen::OutputGenerator;
use crate::icon::meta::{IconTarget, PngTarget};
use crate::icon::IconProcessorError;
use resvg::usvg;
use std::io::Write;

pub fn process_png_target(
    icon: &usvg::Tree,
    target: &IconTarget,
    data: &PngTarget,
    outputs: &OutputGenerator,
) -> Result<(), IconProcessorError> {
    // Render the SVG to a pixmap
    let pixmap = crate::icon::render::render_svg_to_pixmap(
        &resvg::Tree::from_usvg(icon),
        data.width,
        data.height,
    )?;

    // Write the PNG to the output
    let mut writer = outputs.create_target_output(target, "icon.png")?;
    writer.write_all(&pixmap.encode_png()?)?;

    Ok(())
}
