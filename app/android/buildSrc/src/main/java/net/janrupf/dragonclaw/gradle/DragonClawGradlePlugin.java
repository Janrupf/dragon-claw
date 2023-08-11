package net.janrupf.dragonclaw.gradle;

import com.android.build.gradle.BaseExtension;
import net.janrupf.dragonclaw.gradle.extension.DragonClawIconExtension;
import net.janrupf.dragonclaw.gradle.extension.DragonClawIconImport;
import net.janrupf.dragonclaw.gradle.task.DragonClawIconImportTask;
import net.janrupf.dragonclaw.gradle.task.IconImportTaskCreator;
import org.gradle.api.GradleException;
import org.gradle.api.Plugin;
import org.gradle.api.Project;
import org.gradle.api.Task;
import org.gradle.api.tasks.TaskContainer;
import org.gradle.api.tasks.TaskProvider;

import java.io.IOException;
import java.util.Set;

/**
 * Main entry point for the gradle dragon claw icon generator plugin.
 */
public class DragonClawGradlePlugin implements Plugin<Project> {
    private static final String ANDROID_PRE_BUILD_TASK = "preBuild";

    private DragonClawIconExtension extension;

    @Override
    public void apply(Project project) {
        // Register the extension
        this.extension = project.getExtensions().create(
                "dragonClawIcons",
                DragonClawIconExtension.class,
                project
        );

        project.afterEvaluate(this::afterEvaluate);
    }

    private void afterEvaluate(Project project) {
        BaseExtension androidExtension = project.getExtensions().findByType(BaseExtension.class);
        if (androidExtension == null) {
            // We need the android plugin to even to anything sensible
            throw new IllegalStateException("Android plugin not found");
        }

        // Find the pre build task
        TaskContainer tasks = project.getTasks();
        Task preBuildTask = tasks.findByName(ANDROID_PRE_BUILD_TASK);

        TaskProvider<Task> iconImportTask = tasks.register("iconImport");

        for (DragonClawIconImport imp : extension.getIcons()) {
            // Create the tasks for the import
            Set<TaskProvider<DragonClawIconImportTask>> importTasks;
            try {
                importTasks = IconImportTaskCreator.createTasks(project, androidExtension, imp);
            } catch (IOException e) {
                throw new GradleException("Failed to create icon import importTasks", e);
            }

            // Make sure the general icon import task depends on the import tasks
            iconImportTask.configure((t) -> t.dependsOn(importTasks));
        }

        if (preBuildTask != null && !extension.getIcons().isEmpty()) {
            preBuildTask.dependsOn(iconImportTask);
        }
    }
}
