package net.janrupf.dragonclaw.gradle.extension;

import java.io.File;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;

/**
 * A single import of a dragon claw icon for the generator.
 */
public class DragonClawIconImport {
    private final File metaFile;
    private final Set<String> targets;
    private final Set<Object> sourceSets;

    /**
     * Creates a new icon import.
     *
     * @param metaFile the meta file of the icon
     */
    public DragonClawIconImport(File metaFile) {
        this.metaFile = metaFile;
        this.targets = new HashSet<>();
        this.sourceSets = new HashSet<>();
    }

    /**
     * Adds a target to the icon import.
     *
     * @param target the target to add
     */
    public void target(String target) {
        targets.add(target);
    }

    /**
     * Adds targets to the icon import.
     *
     * @param targets the targets to add
     */
    public void targets(String... targets) {
        this.targets.addAll(Arrays.asList(targets));
    }

    /**
     * Adds a source set to the icon import.
     *
     * @param sourceSet the source set to add
     */
    public void sourceSet(Object sourceSet) {
        sourceSets.add(sourceSet);
    }

    /**
     * Adds source sets to the icon import.
     *
     * @param sourceSets the source sets to add
     */
    public void sourceSets(Object... sourceSets) {
        this.sourceSets.addAll(Arrays.asList(sourceSets));
    }

    /**
     * Retrieves the meta file of the icon.
     *
     * @return the meta file of the icon
     */
    public File getMetaFile() {
        return metaFile;
    }

    /**
     * Retrieves the targets to generate.
     *
     * @return the targets to generate
     */
    public Set<String> getTargets() {
        return targets;
    }

    /**
     * Retrieves the source sets to import to.
     *
     * @return the source sets to import to
     */
    public Set<Object> getSourceSets() {
        return sourceSets;
    }
}
