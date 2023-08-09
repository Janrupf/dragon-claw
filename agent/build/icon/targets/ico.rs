use crate::icon::gen::OutputGenerator;
use crate::icon::meta::IcoTarget;
use crate::icon::IconProcessorError;
use resvg::usvg;

pub fn process_ico_target(
    icon: &usvg::Tree,
    data: &IcoTarget,
    outputs: &mut OutputGenerator,
) -> Result<(), IconProcessorError> {
    // Generate a new ICO directory
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for size in &data.sizes {
        // Render the SVG to a Pixmap
        let pixmap =
            crate::icon::render::render_svg_to_pixmap(&resvg::Tree::from_usvg(icon), *size, *size)?;

        let icon_image = ico::IconImage::from_rgba_data(*size, *size, pixmap.take());

        // Add the pixmap to the ICO directory
        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
    }

    // Write the ICO directory to the output
    let mut writer = outputs.create_target_output("icon", "icon.ico")?;
    icon_dir.write(&mut writer)?;

    Ok(())
}
