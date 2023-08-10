use crate::icon::IconProcessorError;
use resvg::{tiny_skia, usvg};

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
pub fn convert_usvg_paint_to_tiny_skia(
    paint: &usvg::Paint,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<tiny_skia::Paint> {
    match paint {
        usvg::Paint::Color(c) => {
            let c = tiny_skia::Color::from_rgba8(c.red, c.green, c.blue, opacity.to_u8());
            Some(tiny_skia::Paint {
                shader: tiny_skia::Shader::SolidColor(c),
                ..Default::default()
            })
        }
        usvg::Paint::LinearGradient(ref lg) => convert_linear_gradient(lg, opacity, object_bbox),
        usvg::Paint::RadialGradient(ref rg) => convert_radial_gradient(rg, opacity, object_bbox),
        usvg::Paint::Pattern(_) => None, /* not supported */
    }
}

fn convert_linear_gradient(
    gradient: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<tiny_skia::Paint> {
    let (mode, transform, points) = convert_base_gradient(gradient, opacity, object_bbox)?;

    let shader = tiny_skia::LinearGradient::new(
        (gradient.x1, gradient.y1).into(),
        (gradient.x2, gradient.y2).into(),
        points,
        mode,
        transform,
    )?;

    Some(tiny_skia::Paint {
        shader,
        ..Default::default()
    })
}

fn convert_radial_gradient(
    gradient: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<tiny_skia::Paint> {
    let (mode, transform, points) = convert_base_gradient(gradient, opacity, object_bbox)?;

    let shader = tiny_skia::RadialGradient::new(
        (gradient.fx, gradient.fy).into(),
        (gradient.cx, gradient.cy).into(),
        gradient.r.get(),
        points,
        mode,
        transform,
    )?;

    Some(tiny_skia::Paint {
        shader,
        ..Default::default()
    })
}

fn convert_base_gradient(
    gradient: &usvg::BaseGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<(
    tiny_skia::SpreadMode,
    tiny_skia::Transform,
    Vec<tiny_skia::GradientStop>,
)> {
    let mode = match gradient.spread_method {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let transform = if gradient.units == usvg::Units::ObjectBoundingBox {
        let bbox = object_bbox?;
        let ts = tiny_skia::Transform::from_bbox(bbox);
        ts.pre_concat(gradient.transform)
    } else {
        gradient.transform
    };

    let mut points = Vec::with_capacity(gradient.stops.len());
    for stop in &gradient.stops {
        let alpha = stop.opacity * opacity;
        let color = tiny_skia::Color::from_rgba8(
            stop.color.red,
            stop.color.green,
            stop.color.blue,
            alpha.to_u8(),
        );
        points.push(tiny_skia::GradientStop::new(stop.offset.get(), color))
    }

    Some((mode, transform, points))
}

#[derive(Debug, Clone)]
pub struct PaintData {
    pub paint: usvg::Paint,
    pub opacity: usvg::Opacity,
}
