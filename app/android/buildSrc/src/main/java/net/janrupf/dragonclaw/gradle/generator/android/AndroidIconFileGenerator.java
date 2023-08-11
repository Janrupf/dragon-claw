package net.janrupf.dragonclaw.gradle.generator.android;

import com.android.ide.common.vectordrawable.Svg2Vector;
import net.janrupf.dragonclaw.gradle.generator.IconFileGenerator;
import net.janrupf.dragonclaw.gradle.meta.android.AndroidIconTargetOptions;
import org.gradle.api.logging.Logger;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;

/**
 * Converts an SVG icon to an Android icon.
 */
public class AndroidIconFileGenerator extends IconFileGenerator {
    private final AndroidIconTargetOptions options;
    private final File drawableDirectory;

    public AndroidIconFileGenerator(
            File metaFile,
            File iconFile,
            File outputDirectory,
            AndroidIconTargetOptions options
    ) {
        super(metaFile, iconFile, outputDirectory);
        this.options = options;
        this.drawableDirectory = new File(outputDirectory, "drawable");
    }


    @Override
    public void generate(Logger logger) throws Exception {
        // Make sure the output directory exists
        if (!drawableDirectory.mkdirs() && !drawableDirectory.isDirectory()) {
            throw new IOException("Failed to create directory " + drawableDirectory.getAbsolutePath());
        }

        File outputFile = new File(drawableDirectory, options.getResourceName() + ".xml");
        try (OutputStream out = new FileOutputStream(outputFile)) {
            // Convert the SVG to XML
            String errors = Svg2Vector.parseSvgToXml(getIconFile().toPath(), out);

            if (errors != null) {
                logger.warn("Failed to cleanly convert SVG to XML: {}", errors);
            }

            out.flush();

            if (outputFile.length() == 0) {
                throw new IOException("Failed to convert SVG to XML");
            }
        } catch (Exception e) {
            if (!outputFile.delete()) {
                logger.warn("Failed to delete partially written file {}", outputFile.getAbsolutePath());
            }

            throw e;
        }
    }
}
