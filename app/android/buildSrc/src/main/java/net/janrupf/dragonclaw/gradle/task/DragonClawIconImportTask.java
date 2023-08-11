package net.janrupf.dragonclaw.gradle.task;

import net.janrupf.dragonclaw.gradle.generator.IconFileGenerator;
import org.gradle.api.DefaultTask;
import org.gradle.api.tasks.*;

import javax.inject.Inject;
import java.io.File;

/**
 * Wrapper around a {@link IconFileGenerator} as a task.
 */
@CacheableTask
public class DragonClawIconImportTask extends DefaultTask {
    private final IconFileGenerator generator;

    /**
     * Creates a new icon import task.
     *
     * @param generator the generator to wrap
     */
    @Inject
    public DragonClawIconImportTask(IconFileGenerator generator) {
        this.generator = generator;
    }

    @InputFile
    @PathSensitive(PathSensitivity.NONE)
    public File getMetaFile() {
        return generator.getMetaFile();
    }

    @InputFile
    @PathSensitive(PathSensitivity.NONE)
    public File getIconFile() {
        return generator.getIconFile();
    }

    @OutputDirectory
    public File getOutputDirectory() {
        return generator.getOutputDirectory();
    }

    @TaskAction
    public void run() throws Exception {
        generator.generate(getLogger());
    }
}
