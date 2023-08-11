package net.janrupf.dragonclaw.gradle.meta.android;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import net.janrupf.dragonclaw.gradle.meta.IconTargetOptions;

/**
 * Options for an android icon target.
 */
public class AndroidIconTargetOptions extends IconTargetOptions {
    private final String resourceName;
    private final String background;
    private final String foreground;

    @JsonCreator
    public AndroidIconTargetOptions(
            @JsonProperty(value = "resourceName", required = true) String resourceName,
            @JsonProperty(value = "background", required = true) String background,
            @JsonProperty(value = "foreground", required = true) String foreground
    ) {
        this.resourceName = resourceName;
        this.background = background;
        this.foreground = foreground;
    }

    /**
     * Retrieves the name of the resource to generate.
     *
     * @return the name of the resource to generate
     */
    public String getResourceName() {
        return resourceName;
    }

    /**
     * Retrieves the id of the background element.
     *
     * @return the id of the background element
     */
    public String getBackground() {
        return background;
    }

    /**
     * Retrieves the id of the foreground element.
     *
     * @return the id of the foreground element
     */
    public String getForeground() {
        return foreground;
    }
}
