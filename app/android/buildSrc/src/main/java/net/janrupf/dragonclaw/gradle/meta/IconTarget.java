package net.janrupf.dragonclaw.gradle.meta;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;
import net.janrupf.dragonclaw.gradle.meta.android.AndroidIconTargetOptions;

public class IconTarget {
    private final String name;
    private final IconTargetOptions options;

    @JsonCreator
    public IconTarget(
            @JsonProperty(value = "name", required = true)
            String name,
            @JsonTypeInfo(
                    use = JsonTypeInfo.Id.NAME,
                    include = JsonTypeInfo.As.EXTERNAL_PROPERTY,
                    property = "type",
                    defaultImpl = Void.class
            )
            @JsonSubTypes(
                    value = {
                            @JsonSubTypes.Type(value = AndroidIconTargetOptions.class, name = "android"),
                    }
            )
            @JsonProperty(value = "options", required = true)
            IconTargetOptions options
    ) {
        this.name = name;
        this.options = options;
    }

    /**
     * Retrieves the name of the target.
     *
     * @return the name of the target
     */
    public String getName() {
        return name;
    }

    /**
     * Retrieves the options of the target.
     *
     * @return the options of the target
     */
    public IconTargetOptions getOptions() {
        return options;
    }
}
