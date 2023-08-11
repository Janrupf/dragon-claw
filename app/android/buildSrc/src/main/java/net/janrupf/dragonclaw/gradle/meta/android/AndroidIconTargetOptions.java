package net.janrupf.dragonclaw.gradle.meta.android;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import net.janrupf.dragonclaw.gradle.meta.IconTargetOptions;

/**
 * Options for an android icon target.
 */
public class AndroidIconTargetOptions extends IconTargetOptions {
    private final String resourceName;

    @JsonCreator
    public AndroidIconTargetOptions(
            @JsonProperty(value = "resourceName", required = true) String resourceName
    ) {
        this.resourceName = resourceName;
    }

    /**
     * Retrieves the name of the resource to generate.
     *
     * @return the name of the resource to generate
     */
    public String getResourceName() {
        return resourceName;
    }
}
