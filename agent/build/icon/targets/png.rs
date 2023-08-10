use crate::icon::gen::OutputGenerator;
use crate::icon::inputs::BuildInputs;
use crate::icon::meta::{DrawStep, PngTarget};
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

    perform_draw_steps(inputs, &mut pixmap.as_mut(), &data.draw_steps.before)?;

    // Render the SVG into the pixmap
    crate::icon::render::render_svg_into_pixmap(
        &resvg::Tree::from_usvg(inputs.icon()),
        &mut pixmap.as_mut(),
        data.placement.x,
        data.placement.y,
        render_width,
        render_height,
    );

    perform_draw_steps(inputs, &mut pixmap.as_mut(), &data.draw_steps.after)?;

    // Write the PNG to the output
    let mut writer = outputs.create_target_output("icon", "icon.png")?;
    writer.write_all(&pixmap.encode_png()?)?;

    Ok(())
}

fn perform_draw_steps(
    inputs: &BuildInputs,
    pixmap: &mut tiny_skia::PixmapMut,
    steps: &[DrawStep],
) -> Result<(), IconProcessorError> {
    for step in steps {
        match step {
            DrawStep::Rect(rect) => {
                let draw_rect = tiny_skia::NonZeroRect::from_xywh(
                    rect.x as _,
                    rect.y as _,
                    rect.width as _,
                    rect.height as _,
                )
                .ok_or(IconProcessorError::InvalidRectangleExtents {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                })?;

                // Attempt to resolve the paint to draw the rect with
                let paint = inputs.resolve_paint_input(&rect.fill)?;

                // Convert the paint
                let paint = crate::icon::render::convert_usvg_paint_to_tiny_skia(
                    &paint.paint,
                    paint.opacity,
                    Some(draw_rect),
                )
                .ok_or(IconProcessorError::PaintConversionFailed)?;

                pixmap.fill_rect(
                    draw_rect.to_rect(),
                    &paint,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
    }

    Ok(())
}
