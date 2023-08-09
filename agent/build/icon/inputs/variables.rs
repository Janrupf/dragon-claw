use crate::icon::meta::IconVariable;
use resvg::usvg;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug)]
pub struct ExtractedVariables {
    variables: HashMap<String, ExtractedVariable>,
}

impl ExtractedVariables {
    /// Extracts the variables from the given SVG tree.
    pub fn extract(
        tree: &usvg::Tree,
        variables: &HashMap<String, IconVariable>,
    ) -> Result<Self, VariableError> {
        let mut extracted_variables = HashMap::new();

        for (name, variable) in variables {
            match variable {
                IconVariable::FillPaint(v) => {
                    // Find the element by id to extract the fill paint from
                    let node = tree
                        .node_by_id(&v.from)
                        .ok_or_else(|| VariableError::ElementNotFound(name.clone()))?;

                    let fill = match &*node.borrow() {
                        usvg::NodeKind::Path(p) => p.fill.clone(),
                        kind => {
                            // The variable is not compatible with the element type
                            return Err(VariableError::IncompatibleElementKind {
                                id: v.from.clone(),
                                node_kind: kind.clone(),
                                variable_type: VariableType::Paint,
                            });
                        }
                    };

                    // Extract the real fill if possible
                    let fill = fill.ok_or_else(|| VariableError::ElementMissingAttribute {
                        id: v.from.clone(),
                        attribute: "fill".to_owned(),
                        variable_type: VariableType::Paint,
                    })?;

                    // Save the variable
                    extracted_variables.insert(
                        name.clone(),
                        ExtractedVariable::Paint(PaintVariable {
                            paint: fill.paint,
                            opacity: fill.opacity,
                        }),
                    );
                }
            }
        }

        Ok(Self {
            variables: extracted_variables,
        })
    }

    /// Retrieves a paint variable by name
    pub fn paint(&self, name: &str) -> Result<&PaintVariable, VariableError> {
        #[allow(unreachable_patterns)] // Currently we only have one variable type
        match self.variables.get(name) {
            Some(ExtractedVariable::Paint(v)) => Ok(v),
            Some(v) => Err(VariableError::IncompatibleVariableType {
                expected: VariableType::Paint,
                found: v.ty(),
            }),
            None => Err(VariableError::NotFound(name.to_owned())),
        }
    }
}

/// A variable extracted from the build inputs
#[derive(Debug)]
pub enum ExtractedVariable {
    Paint(PaintVariable),
}

#[derive(Debug, Clone)]
pub struct PaintVariable {
    paint: usvg::Paint,
    opacity: usvg::Opacity,
}

impl ExtractedVariable {
    /// Returns the type of the variable
    pub fn ty(&self) -> VariableType {
        match self {
            Self::Paint { .. } => VariableType::Paint,
        }
    }
}

/// The type of a variable
#[derive(Debug)]
pub enum VariableType {
    Paint,
}

#[derive(Debug, Error)]
pub enum VariableError {
    #[error("element not found: {0}")]
    ElementNotFound(String),

    #[error(
        "cannot extract variable of type {variable_type:?} from element {id} of type {node_kind:?}"
    )]
    IncompatibleElementKind {
        id: String,
        node_kind: usvg::NodeKind,
        variable_type: VariableType,
    },

    #[error("cannot extract variable of type {variable_type:?} from element {id} because it is missing the {attribute} attribute")]
    ElementMissingAttribute {
        id: String,
        attribute: String,
        variable_type: VariableType,
    },

    #[error("expected variable of type {expected:?} but found variable of type {found:?}")]
    IncompatibleVariableType {
        expected: VariableType,
        found: VariableType,
    },

    #[error("variable not found: {0}")]
    NotFound(String),
}
