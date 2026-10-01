//! Typed failure taxonomy for MAMUT Math core contracts.

use core::fmt;

/// Exhaustive classification of failures raised by the contract layer.
///
/// The taxonomy is deliberately small and stable. Structural payload
/// corruption that has no more specific code maps to [`ErrorKind::Reserved`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// A floating-point value was `NaN` or infinite where a finite value is required.
    InvalidNotFinite,
    /// An ordering or inequality contract was violated (for example `lower > upper`).
    InvertedBounds,
    /// A collection that is required to be non-empty was empty.
    EmptyCollection,
    /// Two collections that must agree in length disagreed.
    DimensionMismatch,
    /// A value fell outside the domain declared by a variable.
    DomainMismatch,
    /// A value was not a member of the declared discrete set.
    ValueNotInDiscreteSet,
    /// An index fell outside the allowed categorical/index range.
    IndexOutOfRange,
    /// The serialized schema version is not supported by this decoder.
    UnsupportedSchemaVersion,
    /// The serialized payload ended before a complete value could be read.
    TruncatedPayload,
    /// The serialized payload contained unexpected trailing bytes.
    TrailingPayload,
    /// A failure without a more specific classification, including structural
    /// payload corruption and invalid UTF-8 inside string payloads.
    Reserved,
}

/// Structured error carrying an [`ErrorKind`], a message and optional context.
///
/// [`fmt::Display`] and [`std::error::Error`] are implemented by hand; the
/// crate has no external dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathError {
    kind: ErrorKind,
    message: String,
    context: Vec<String>,
}

impl MathError {
    /// Create an error from a kind and a message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
        }
    }

    /// Return a copy of this error with one more context entry appended.
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context.push(context.into());
        self
    }

    /// The error classification.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Additional context entries, in insertion order.
    #[must_use]
    pub fn context(&self) -> &[String] {
        &self.context
    }
}

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)?;
        if !self.context.is_empty() {
            write!(f, " [{}]", self.context.join(" | "))?;
        }
        Ok(())
    }
}

impl std::error::Error for MathError {}
