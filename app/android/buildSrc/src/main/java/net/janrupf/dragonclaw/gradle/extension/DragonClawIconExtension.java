package net.janrupf.dragonclaw.gradle.extension;

import org.gradle.api.Action;
import org.gradle.api.Project;

import javax.inject.Inject;
import java.io.File;
import java.util.HashSet;
import java.util.Set;

/**
 * Configuration for the DragonClaw icon generator.
 */
public class DragonClawIconExtension {
    private final Project project;
    private final Set<DragonClawIconImport> icons;

    @Inject
    public DragonClawIconExtension(Project project) {
        this.project = project;
        this.icons = new HashSet<>();
    }

    /**
     * Adds an icon to the icon generator.
     *
     * @param icon the icon to add
     */
    public void icon(DragonClawIconImport icon) {
        icons.add(icon);
    }

    /**
     * Adds an icon to the icon generator.
     *
     * @param metaFile the meta file of the icon
     * @param configurator the configuration for the icon
     */
    public void icon(Object metaFile, Action<DragonClawIconImport> configurator) {
        File file = project.file(metaFile);

        DragonClawIconImport icon = new DragonClawIconImport(file);
        configurator.execute(icon);

        icons.add(icon);
    }

    /**
     * Retrieves the icons to generate.
     *
     * @return the icons to generate
     */
    public Set<DragonClawIconImport> getIcons() {
        return icons;
    }
}
