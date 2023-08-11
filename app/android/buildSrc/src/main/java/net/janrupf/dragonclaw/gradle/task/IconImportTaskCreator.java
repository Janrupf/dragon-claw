package net.janrupf.dragonclaw.gradle.task;

import com.android.build.api.dsl.AndroidSourceSet;
import com.android.build.gradle.BaseExtension;
import com.fasterxml.jackson.databind.ObjectMapper;
import net.janrupf.dragonclaw.gradle.extension.DragonClawIconImport;
import net.janrupf.dragonclaw.gradle.generator.IconFileGenerator;
import net.janrupf.dragonclaw.gradle.meta.IconMeta;
import net.janrupf.dragonclaw.gradle.meta.IconTarget;
import net.janrupf.dragonclaw.gradle.meta.IconTargetOptions;
import org.gradle.api.Project;
import org.gradle.api.tasks.SourceSet;
import org.gradle.api.tasks.TaskProvider;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.Set;

/**
 * Helper class for creating icon import tasks.
 */
public class IconImportTaskCreator {
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    private IconImportTaskCreator() {
        throw new RuntimeException("Utility class");
    }

    /**
     * Creates new icon import tasks for the given import.
     *
     * @param project          the project to create the task for
     * @param androidExtension the android extension to create the task for
     * @param imp              the import to create the task for
     * @return the created task
     */
    public static Set<TaskProvider<DragonClawIconImportTask>> createTasks(
            Project project,
            BaseExtension androidExtension,
            DragonClawIconImport imp
    ) throws IOException {
        if (imp.getTargets().isEmpty()) {
            throw new IllegalArgumentException("No targets specified for icon import " + imp.getMetaFile());
        }

        File metaFile = imp.getMetaFile();
        Path resourceDirectory = metaFile.getParentFile().toPath();

        // Read in the meta file
        IconMeta meta;
        try (FileInputStream in = new FileInputStream(metaFile)) {
            meta = OBJECT_MAPPER.readValue(in, IconMeta.class);
        }

        File iconFile = resourceDirectory.resolve(meta.getFile()).toFile();
        if (!iconFile.exists()) {
            throw new NoSuchFileException("Icon file " + iconFile + " does not exist");
        }

        Set<TaskProvider<DragonClawIconImportTask>> tasks = new HashSet<>();
        for (String targetName : imp.getTargets()) {
            File outputDirectory = new File(project.getBuildDir(), "generated/dragonclaw/icons/" + targetName);

            // Find the options for the target
            IconTargetOptions options = null;
            for (IconTarget target : meta.getTargets()) {
                if (target.getName().equals(targetName)) {
                    options = target.getOptions();
                    break;
                }
            }

            if (options == null) {
                throw new IllegalArgumentException("No such target " + targetName);
            }

            // Create the generator
            IconFileGenerator generator = IconFileGenerator.createFor(
                    options,
                    resourceDirectory,
                    metaFile,
                    iconFile,
                    outputDirectory);

            // Get the name of the task
            String taskName = "generateIcon" + targetName;

            tasks.add(project.getTasks().register(
                    taskName,
                    DragonClawIconImportTask.class,
                    generator
            ));

            // Make sure the source set depends on the task
            for (Object maybeSourceSet : imp.getSourceSets()) {
                // Somehow translate the source set to a real source set
                AndroidSourceSet sourceSet;
                if (maybeSourceSet instanceof AndroidSourceSet) {
                    sourceSet = (AndroidSourceSet) maybeSourceSet;
                } else if (maybeSourceSet instanceof SourceSet) {
                    sourceSet = androidExtension.getSourceSets().getByName(((SourceSet) maybeSourceSet).getName());
                } else if (maybeSourceSet instanceof CharSequence) {
                    sourceSet = androidExtension.getSourceSets().getByName(maybeSourceSet.toString());
                } else {
                    throw new IllegalArgumentException(
                            "Can not resolve an object of type " + maybeSourceSet.getClass().getName() + " to a source set"
                    );
                }

                // Add the generated output directory to the source set
                sourceSet.getRes().srcDir(outputDirectory);
            }
        }

        return tasks;
    }
}
