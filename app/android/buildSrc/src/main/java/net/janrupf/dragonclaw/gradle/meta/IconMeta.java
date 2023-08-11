package net.janrupf.dragonclaw.gradle.meta;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

import java.nio.file.Path;
import java.util.Set;

@JsonIgnoreProperties(
        value = {"variables"}
)
public class IconMeta {
    private final Path file;
    private final Set<IconTarget> targets;

    @JsonCreator
    public IconMeta(
            @JsonProperty(value = "file", required = true) Path file,
            @JsonProperty(value = "targets", required = true) Set<IconTarget> targets
    ) {
        this.file = file;
        this.targets = targets;
    }

    /**
     * Retrieves the file of the icon.
     *
     * @return the file of the icon
     */
    public Path getFile() {
        return file;
    }

    /**
     * Retrieves the targets this icon declares.
     *
     * @return the targets this icon declares
     */
    public Set<IconTarget> getTargets() {
        return targets;
    }
}
