use crate::icon::inputs::variables::ExtractedVariables;
use crate::icon::meta::IconMetadata;
use crate::icon::IconProcessorError;
use resvg::usvg;

pub mod variables;

pub struct BuildInputs {
    icon: usvg::Tree,
    variables: ExtractedVariables,
}

impl BuildInputs {
    /// Instantiates the build inputs from the given icon.
    pub fn instantiate(
        icon: usvg::Tree,
        metadata: &IconMetadata,
    ) -> Result<Self, IconProcessorError> {
        let variables = ExtractedVariables::extract(&icon, &metadata.variables)?;
        Ok(Self { icon, variables })
    }

    
    /// Retrieves the icon.
    pub fn icon(&self) -> &usvg::Tree {
        &self.icon
    }
    
    /// Retrieves the extracted variables.
    pub fn variables(&self) -> &ExtractedVariables {
        &self.variables
    }
}
