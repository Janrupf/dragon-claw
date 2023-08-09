use crate::icon::IconProcessorError;
use resvg::tiny_skia;

pub fn render_svg_into_pixmap(
    render_tree: &resvg::Tree,
    pixmap: &mut tiny_skia::PixmapMut,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    // Compute the scale factor
    let x_scale = width as f32 / render_tree.size.width();
    let y_scale = height as f32 / render_tree.size.height();

    let transform = tiny_skia::Transform {
        sx: x_scale,
        sy: y_scale,
        tx: x as f32,
        ty: y as f32,
        ..Default::default()
    };

    // Render the SVG to the pixmap
    render_tree.render(transform, pixmap);
}

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

    render_svg_into_pixmap(
        render_tree,
        &mut pixmap.as_mut(),
        0,
        0,
        target_width,
        target_height,
    );

    Ok(pixmap)
}
