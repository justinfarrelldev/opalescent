use crate::type_system::{errors::TypeError, type_mapping::AstTypeMappingError};

impl From<AstTypeMappingError> for TypeError {
    fn from(value: AstTypeMappingError) -> Self {
        match value {
            AstTypeMappingError::TypeNotFound { type_name, span } => Self::TypeNotFound {
                type_name,
                span: Self::span_from_span(span),
            },
        }
    }
}
