package net.janrupf.dragonclaw.gradle.generator;

import net.janrupf.dragonclaw.gradle.generator.android.AndroidIconFileGenerator;
import net.janrupf.dragonclaw.gradle.meta.IconTargetOptions;
import net.janrupf.dragonclaw.gradle.meta.android.AndroidIconTargetOptions;
import org.gradle.api.logging.Logger;

import java.io.File;
import java.nio.file.Path;

/**
 * Base class for icon file generators.
 */
public abstract class IconFileGenerator {
    private final File metaFile;
    private final File iconFile;
    private final File outputDirectory;

    protected IconFileGenerator(File metaFile, File iconFile, File outputDirectory) {
        this.metaFile = metaFile;
        this.iconFile = iconFile;
        this.outputDirectory = outputDirectory;
    }

    /**
     * Retrieves the meta file that describes the icon.
     *
     * @return the meta file that describes the icon
     */
    public final File getMetaFile() {
        return metaFile;
    }

    /**
     * Retrieves the original icon file.
     *
     * @return the original icon file
     */
    public final File getIconFile() {
        return iconFile;
    }

    /**
     * Retrieves the output directory the generator will write to.
     *
     * @return the output directory
     */
    public final File getOutputDirectory() {
        return outputDirectory;
    }

    /**
     * Runs the generator.
     *
     * @param logger the logger to log to
     * @throws Exception if an error occurs
     */
    public abstract void generate(Logger logger) throws Exception;

    /**
     * Creates a new generator for the given options.
     *
     * @param options the options to create the generator for
     * @param resourceDirectory the directory to resolve meta resources relative to
     * @param metaFile the meta file the options come from
     * @param iconFile the icon file to generate from
     * @param outputDirectory the output directory to write to
     * @return the created generator
     */
    public static IconFileGenerator createFor(
            IconTargetOptions options,
            Path resourceDirectory,
            File metaFile,
            File iconFile,
            File outputDirectory
    ) {
        if (options instanceof AndroidIconTargetOptions) {
            return new AndroidIconFileGenerator(
                    metaFile,
                    iconFile,
                    outputDirectory,
                    (AndroidIconTargetOptions) options
            );
        }

        throw new UnsupportedOperationException("Unsupported target options of type " + options.getClass().getName());
    }
}
