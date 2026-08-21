use super::Warning;

impl Warning {
    /// Return the suppression annotation attached to this warning, if present.
    pub fn suppression_annotation(&self) -> Option<&str> {
        let suppression_annotation = match *self {
            Self::ArithmeticOverflow {
                ref suppression_annotation,
                ..
            }
            | Self::UnsafeCast {
                ref suppression_annotation,
                ..
            }
            | Self::UnusedVariable {
                ref suppression_annotation,
                ..
            }
            | Self::UnreachableCode {
                ref suppression_annotation,
                ..
            }
            | Self::ReplaceableErrorList {
                ref suppression_annotation,
                ..
            }
            | Self::NonExhaustiveMatch {
                ref suppression_annotation,
                ..
            } => suppression_annotation,
        };
        suppression_annotation.as_deref()
    }
}
