//! Opaque identifier for a single numerical run.

/// A caller-supplied run identity.
///
/// The identifier is opaque data: MAMUT never interprets its structure. It is a
/// `u128` so that upstream systems can pack a generation counter, a timestamp or
/// a content hash without core having to know which.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId(u128);

impl RunId {
    /// Wrap a raw `u128` run identity.
    ///
    /// This is infallible: every `u128` is a valid identity.
    #[must_use]
    pub fn new(value: u128) -> Self {
        Self(value)
    }

    /// The raw `u128` value.
    #[must_use]
    pub fn value(self) -> u128 {
        self.0
    }
}
