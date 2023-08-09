use crate::icon::gen::OutputGenerator;
use crate::icon::inputs::BuildInputs;
use crate::icon::meta::PngTarget;
use crate::icon::IconProcessorError;
use resvg::tiny_skia;
use std::io::Write;

pub fn process_png_target(
    inputs: &BuildInputs,
    data: &PngTarget,
    outputs: &mut OutputGenerator,
) -> Result<(), IconProcessorError> {
    // Construct a pixmap to render to
    let mut pixmap = tiny_skia::Pixmap::new(data.width, data.height).ok_or(
        IconProcessorError::InvalidPixmapDimensions {
            width: data.width,
            height: data.height,
        },
    )?;

    let render_width = data.placement.width.unwrap_or(data.width);
    let render_height = data.placement.height.unwrap_or(data.height);

    // Render the SVG into the pixmap
    crate::icon::render::render_svg_into_pixmap(
        &resvg::Tree::from_usvg(inputs.icon()),
        &mut pixmap.as_mut(),
        data.placement.x,
        data.placement.y,
        render_width,
        render_height,
    );

    // Write the PNG to the output
    let mut writer = outputs.create_target_output("icon", "icon.png")?;
    writer.write_all(&pixmap.encode_png()?)?;

    Ok(())
}
