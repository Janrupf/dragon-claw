use crate::icon::IconProcessorError;
use resvg::tiny_skia;

pub fn render_svg_to_pixmap(
    render_tree: &resvg::Tree,
    target_width: u32,
    target_height: u32,
) -> Result<tiny_skia::Pixmap, IconProcessorError> {
    // Construct a pixmap to render to
    let mut pixmap = tiny_skia::Pixmap::new(target_width, target_height).ok_or(
        IconProcessorError::InvalidPixmapDimensions {
            width: target_width,
            height: target_height,
        },
    )?;

    // Compute the scale factor
    let x_scale = target_width as f32 / render_tree.size.width();
    let y_scale = target_height as f32 / render_tree.size.height();

    let transform = tiny_skia::Transform {
        sx: x_scale,
        sy: y_scale,
        ..Default::default()
    };

    // Render the SVG to the pixmap
    render_tree.render(transform, &mut pixmap.as_mut());

    Ok(pixmap)
}
