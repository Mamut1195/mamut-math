//! Canonical, deterministic, hand-written serialization for the core contracts.
//!
//! ## Format rules
//!
//! * All integers are little-endian.
//! * Enum discriminants are single `u8` tags numbered from `0` in declaration
//!   order of each enum.
//! * Collection lengths are `u32`; where a collection must be non-empty the
//!   decoder rejects a zero length with [`ErrorKind::EmptyCollection`].
//! * Struct/enum payload fields are written in declaration order and are never
//!   reordered between schema versions.
//! * `f64` values are written as their exact IEEE-754 bit pattern
//!   (`f64::to_bits`, little-endian); decoding re-validates finiteness.
//! * A `String` is a `u32` byte length followed by its UTF-8 bytes.
//! * [`usize`] indices are written as `u32`.
//!
//! Every top-level public helper writes a 4-byte [`SCHEMA_VERSION`] header, and
//! the decoder rejects any other version with
//! [`ErrorKind::UnsupportedSchemaVersion`]. Decoding is total: short reads yield
//! [`ErrorKind::TruncatedPayload`], leftover bytes after a complete top-level
//! value yield [`ErrorKind::TrailingPayload`], non-finite float bits yield
//! [`ErrorKind::InvalidNotFinite`], and every value is rebuilt through its
//! validating constructor so invalid bounds or empty collections cannot be
//! introduced by decoding.
//!
//! ## Fallible encoding
//!
//! The wire format stores lengths and indices as `u32` (see [`SCHEMA_VERSION`]).
//! Values whose `usize` magnitude does not fit in `u32` — a collection longer
//! than `u32::MAX`, an index above `u32::MAX`, or a string longer than
//! `u32::MAX` bytes — cannot be represented and are therefore **rejected
//! explicitly** with [`ErrorKind::IndexOutOfRange`] rather than silently
//! truncated (the kind is reused from the taxonomy to mean "serialized byte
//! value beyond the wire-format range"; no new taxonomy member is introduced).
//! Consequently the public `encode_*` helpers and the internal
//! `Canonical::encode` method are fallible. Decoding is unchanged, so the
//! byte format and [`SCHEMA_VERSION`] remain identical to v1.
//!
//! Determinism: identical values encode to byte-identical output. There is no
//! map iteration and no floating-point formatting involved.

use crate::constraint::{ConstraintDescriptor, ConstraintKind, ConstraintViolation};
use crate::error::{ErrorKind, MathError};
use crate::evaluation::{
    Candidate, Diagnostic, DiagnosticCode, Diagnostics, EvaluationOutcome, Feasibility,
};
use crate::objective::{ObjectiveDescriptor, ObjectiveSense, ObjectiveValues};
use crate::value::ValidFloat;
use crate::variable::{DiscreteSet, Variable, VariableId, VariableKind};

/// Version of the canonical byte codec.
pub const SCHEMA_VERSION: u32 = 1;

/// Internal codec implemented by every serializable contract type.
///
/// Implementations write only their own payload (no version header); the
/// top-level helpers in this module add and check the header. Encoding is
/// fallible because `usize` magnitudes that do not fit the `u32` wire format
/// must fail explicitly instead of truncating.
trait Canonical: Sized {
    /// Append this value's canonical payload to `out`.
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError>;
    /// Read a value from `dec`, rebuilding it through validating constructors.
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError>;
}

/// Cursor-based, bounds-checked reader over a byte slice.
struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn truncated() -> MathError {
        MathError::new(
            ErrorKind::TruncatedPayload,
            "payload ended before a complete value could be read",
        )
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], MathError> {
        let end = self.pos.checked_add(n).ok_or_else(Self::truncated)?;
        if end > self.bytes.len() {
            return Err(Self::truncated());
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, MathError> {
        Ok(self.take(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, MathError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().expect("exactly 4 bytes");
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, MathError> {
        let bytes: [u8; 8] = self.take(8)?.try_into().expect("exactly 8 bytes");
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_i64(&mut self) -> Result<i64, MathError> {
        let bytes: [u8; 8] = self.take(8)?.try_into().expect("exactly 8 bytes");
        Ok(i64::from_le_bytes(bytes))
    }

    fn read_valid_float(&mut self) -> Result<ValidFloat, MathError> {
        let bits = self.read_u64()?;
        ValidFloat::new(f64::from_bits(bits))
    }

    fn read_string(&mut self) -> Result<String, MathError> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        core::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| MathError::new(ErrorKind::Reserved, "invalid UTF-8 in string payload"))
    }
}

fn write_u32(value: u32, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(value: u64, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i64(value: i64, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Convert a `usize` magnitude to the `u32` wire format, rejecting values that
/// do not fit instead of truncating.
///
/// `what` names the offending field for the error message.
fn checked_u32(value: usize, what: &str) -> Result<u32, MathError> {
    u32::try_from(value).map_err(|_| {
        MathError::new(
            ErrorKind::IndexOutOfRange,
            format!("{what} exceeds the u32 wire-format range"),
        )
    })
}

/// Write a `u32` collection length, rejecting a length above `u32::MAX`.
fn write_count(count: usize, out: &mut Vec<u8>) -> Result<(), MathError> {
    write_u32(checked_u32(count, "collection length")?, out);
    Ok(())
}

/// Write a length-prefixed UTF-8 string, rejecting a byte length above `u32::MAX`.
fn write_string(value: &str, out: &mut Vec<u8>) -> Result<(), MathError> {
    write_u32(checked_u32(value.len(), "string byte length")?, out);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

/// Write a `u32`-counted list of indices, rejecting any index above `u32::MAX`.
fn write_indices(indices: &[usize], out: &mut Vec<u8>) -> Result<(), MathError> {
    write_count(indices.len(), out)?;
    for index in indices {
        write_u32(checked_u32(*index, "index")?, out);
    }
    Ok(())
}

fn read_indices(dec: &mut Decoder<'_>) -> Result<Vec<usize>, MathError> {
    let count = dec.read_u32()?;
    let mut indices = Vec::new();
    for _ in 0..count {
        indices.push(dec.read_u32()? as usize);
    }
    Ok(indices)
}

fn decode_values<T: Canonical>(
    dec: &mut Decoder<'_>,
    require_non_empty: bool,
    empty_message: &str,
) -> Result<Vec<T>, MathError> {
    let count = dec.read_u32()?;
    if require_non_empty && count == 0 {
        return Err(MathError::new(ErrorKind::EmptyCollection, empty_message));
    }
    let mut values = Vec::new();
    for _ in 0..count {
        values.push(T::decode(dec)?);
    }
    Ok(values)
}

impl Canonical for ValidFloat {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u64(self.get().to_bits(), out);
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        dec.read_valid_float()
    }
}

impl Canonical for VariableId {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u64(self.0, out);
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        Ok(VariableId(dec.read_u64()?))
    }
}

impl Canonical for DiscreteSet {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_count(self.len(), out)?;
        for value in self.values() {
            value.encode(out)?;
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let values = decode_values(dec, true, "discrete set must not be empty")?;
        DiscreteSet::new(values)
    }
}

impl Canonical for VariableKind {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            VariableKind::Continuous { lower, upper } => {
                out.push(0);
                lower.encode(out)?;
                upper.encode(out)?;
            }
            VariableKind::Integer { lower, upper } => {
                out.push(1);
                write_i64(*lower, out);
                write_i64(*upper, out);
            }
            VariableKind::Discrete { values } => {
                out.push(2);
                values.encode(out)?;
            }
            VariableKind::Categorical { cardinality } => {
                out.push(3);
                write_u64(*cardinality, out);
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => VariableKind::continuous(ValidFloat::decode(dec)?, ValidFloat::decode(dec)?),
            1 => VariableKind::integer(dec.read_i64()?, dec.read_i64()?),
            2 => VariableKind::discrete(DiscreteSet::decode(dec)?),
            3 => VariableKind::categorical(dec.read_u64()?),
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown VariableKind tag",
            )),
        }
    }
}

impl Canonical for Variable {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        self.id().encode(out)?;
        self.kind().encode(out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let id = VariableId::decode(dec)?;
        let kind = VariableKind::decode(dec)?;
        Variable::new(id, kind)
    }
}

impl Canonical for ObjectiveSense {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        out.push(match self {
            ObjectiveSense::Minimize => 0,
            ObjectiveSense::Maximize => 1,
        });
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(ObjectiveSense::Minimize),
            1 => Ok(ObjectiveSense::Maximize),
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown ObjectiveSense tag",
            )),
        }
    }
}

impl Canonical for ObjectiveDescriptor {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u32(
            checked_u32(self.index(), "objective descriptor index")?,
            out,
        );
        self.sense().encode(out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let index = dec.read_u32()? as usize;
        let sense = ObjectiveSense::decode(dec)?;
        Ok(ObjectiveDescriptor::new(index, sense))
    }
}

impl Canonical for ObjectiveValues {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_count(self.len(), out)?;
        for value in self.iter() {
            value.encode(out)?;
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let values = decode_values(dec, true, "objective vector must not be empty")?;
        ObjectiveValues::new(values)
    }
}

impl Canonical for ConstraintKind {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            ConstraintKind::Hard => out.push(0),
            ConstraintKind::Soft { tolerance } => {
                out.push(1);
                tolerance.encode(out)?;
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(ConstraintKind::Hard),
            1 => ConstraintKind::soft(ValidFloat::decode(dec)?),
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown ConstraintKind tag",
            )),
        }
    }
}

impl Canonical for ConstraintDescriptor {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u32(
            checked_u32(self.index(), "constraint descriptor index")?,
            out,
        );
        self.kind().encode(out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let index = dec.read_u32()? as usize;
        let kind = ConstraintKind::decode(dec)?;
        ConstraintDescriptor::new(index, kind)
    }
}

impl Canonical for ConstraintViolation {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        self.magnitude().encode(out)
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        ConstraintViolation::new(ValidFloat::decode(dec)?)
    }
}

impl Canonical for Feasibility {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            Feasibility::Feasible => out.push(0),
            Feasibility::InfeasibleHard { violated } => {
                out.push(1);
                write_indices(violated, out)?;
            }
            Feasibility::SoftViolation { violated } => {
                out.push(2);
                write_indices(violated, out)?;
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(Feasibility::Feasible),
            1 => Ok(Feasibility::InfeasibleHard {
                violated: read_indices(dec)?,
            }),
            2 => Ok(Feasibility::SoftViolation {
                violated: read_indices(dec)?,
            }),
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown Feasibility tag",
            )),
        }
    }
}

impl Canonical for DiagnosticCode {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        out.push(match self {
            DiagnosticCode::DegenerateFront => 0,
            DiagnosticCode::EvaluationWarning => 1,
            DiagnosticCode::Reserved => 2,
        });
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(DiagnosticCode::DegenerateFront),
            1 => Ok(DiagnosticCode::EvaluationWarning),
            2 => Ok(DiagnosticCode::Reserved),
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown DiagnosticCode tag",
            )),
        }
    }
}

impl Canonical for Diagnostic {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        self.code.encode(out)?;
        write_string(&self.note, out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let code = DiagnosticCode::decode(dec)?;
        let note = dec.read_string()?;
        Ok(Diagnostic { code, note })
    }
}

impl Canonical for Diagnostics {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_count(self.len(), out)?;
        for diagnostic in self.iter() {
            diagnostic.encode(out)?;
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let items = decode_values(dec, false, "diagnostics")?;
        Ok(Diagnostics::new(items))
    }
}

impl Canonical for ErrorKind {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        out.push(match self {
            ErrorKind::InvalidNotFinite => 0,
            ErrorKind::InvertedBounds => 1,
            ErrorKind::EmptyCollection => 2,
            ErrorKind::DimensionMismatch => 3,
            ErrorKind::DomainMismatch => 4,
            ErrorKind::ValueNotInDiscreteSet => 5,
            ErrorKind::IndexOutOfRange => 6,
            ErrorKind::UnsupportedSchemaVersion => 7,
            ErrorKind::TruncatedPayload => 8,
            ErrorKind::TrailingPayload => 9,
            ErrorKind::Reserved => 10,
        });
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(ErrorKind::InvalidNotFinite),
            1 => Ok(ErrorKind::InvertedBounds),
            2 => Ok(ErrorKind::EmptyCollection),
            3 => Ok(ErrorKind::DimensionMismatch),
            4 => Ok(ErrorKind::DomainMismatch),
            5 => Ok(ErrorKind::ValueNotInDiscreteSet),
            6 => Ok(ErrorKind::IndexOutOfRange),
            7 => Ok(ErrorKind::UnsupportedSchemaVersion),
            8 => Ok(ErrorKind::TruncatedPayload),
            9 => Ok(ErrorKind::TrailingPayload),
            10 => Ok(ErrorKind::Reserved),
            _ => Err(MathError::new(ErrorKind::Reserved, "unknown ErrorKind tag")),
        }
    }
}

impl Canonical for MathError {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        self.kind().encode(out)?;
        write_string(self.message(), out)?;
        let context = self.context();
        write_count(context.len(), out)?;
        for entry in context {
            write_string(entry, out)?;
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let kind = ErrorKind::decode(dec)?;
        let message = dec.read_string()?;
        let count = dec.read_u32()?;
        let mut error = MathError::new(kind, message);
        for _ in 0..count {
            error = error.with_context(dec.read_string()?);
        }
        Ok(error)
    }
}

impl Canonical for Candidate {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_count(self.len(), out)?;
        for value in self.as_slice() {
            value.encode(out)?;
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let values = decode_values(dec, true, "candidate must not be empty")?;
        Candidate::new(values)
    }
}

impl Canonical for EvaluationOutcome {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            EvaluationOutcome::Success {
                objectives,
                constraints,
                feasibility,
                diagnostics,
            } => {
                out.push(0);
                objectives.encode(out)?;
                write_count(constraints.len(), out)?;
                for violation in constraints {
                    violation.encode(out)?;
                }
                feasibility.encode(out)?;
                diagnostics.encode(out)?;
            }
            EvaluationOutcome::Failure { error } => {
                out.push(1);
                error.encode(out)?;
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => {
                let objectives = ObjectiveValues::decode(dec)?;
                let constraints = decode_values::<ConstraintViolation>(dec, false, "constraints")?;
                let feasibility = Feasibility::decode(dec)?;
                let diagnostics = Diagnostics::decode(dec)?;
                Ok(EvaluationOutcome::Success {
                    objectives,
                    constraints,
                    feasibility,
                    diagnostics,
                })
            }
            1 => {
                let error = MathError::decode(dec)?;
                Ok(EvaluationOutcome::Failure { error })
            }
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown EvaluationOutcome tag",
            )),
        }
    }
}

fn encode_with_header<T: Canonical>(value: &T) -> Result<Vec<u8>, MathError> {
    let mut out = Vec::new();
    write_u32(SCHEMA_VERSION, &mut out);
    value.encode(&mut out)?;
    Ok(out)
}

fn decode_with_header<T: Canonical>(bytes: &[u8]) -> Result<T, MathError> {
    let mut dec = Decoder::new(bytes);
    read_version(&mut dec)?;
    let value = T::decode(&mut dec)?;
    ensure_consumed(&dec)?;
    Ok(value)
}

fn encode_slice_with_header<T: Canonical>(items: &[T]) -> Result<Vec<u8>, MathError> {
    let mut out = Vec::new();
    write_u32(SCHEMA_VERSION, &mut out);
    write_count(items.len(), &mut out)?;
    for item in items {
        item.encode(&mut out)?;
    }
    Ok(out)
}

fn decode_slice_with_header<T: Canonical>(bytes: &[u8]) -> Result<Vec<T>, MathError> {
    let mut dec = Decoder::new(bytes);
    read_version(&mut dec)?;
    let values = decode_values::<T>(&mut dec, false, "items")?;
    ensure_consumed(&dec)?;
    Ok(values)
}

fn read_version(dec: &mut Decoder<'_>) -> Result<(), MathError> {
    let version = dec.read_u32()?;
    if version != SCHEMA_VERSION {
        return Err(MathError::new(
            ErrorKind::UnsupportedSchemaVersion,
            "unsupported schema version",
        ));
    }
    Ok(())
}

fn ensure_consumed(dec: &Decoder<'_>) -> Result<(), MathError> {
    if dec.is_empty() {
        Ok(())
    } else {
        Err(MathError::new(
            ErrorKind::TrailingPayload,
            "unexpected trailing bytes after a complete value",
        ))
    }
}

/// Encode a slice of variables with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if the slice length or any variable payload
/// exceeds the `u32` wire-format range.
pub fn encode_variables(variables: &[Variable]) -> Result<Vec<u8>, MathError> {
    encode_slice_with_header(variables)
}

/// Decode a slice of variables produced by [`encode_variables`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_variables(bytes: &[u8]) -> Result<Vec<Variable>, MathError> {
    decode_slice_with_header(bytes)
}

/// Encode a candidate with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if a payload value exceeds the `u32` wire-format range.
pub fn encode_candidate(candidate: &Candidate) -> Result<Vec<u8>, MathError> {
    encode_with_header(candidate)
}

/// Decode a candidate produced by [`encode_candidate`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_candidate(bytes: &[u8]) -> Result<Candidate, MathError> {
    decode_with_header(bytes)
}

/// Encode an objective vector with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if the vector length exceeds the `u32` wire-format range.
pub fn encode_objective_values(values: &ObjectiveValues) -> Result<Vec<u8>, MathError> {
    encode_with_header(values)
}

/// Decode an objective vector produced by [`encode_objective_values`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_objective_values(bytes: &[u8]) -> Result<ObjectiveValues, MathError> {
    decode_with_header(bytes)
}

/// Encode a diagnostics collection with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if the collection length or a note string
/// exceeds the `u32` wire-format range.
pub fn encode_diagnostics(diagnostics: &Diagnostics) -> Result<Vec<u8>, MathError> {
    encode_with_header(diagnostics)
}

/// Decode a diagnostics collection produced by [`encode_diagnostics`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_diagnostics(bytes: &[u8]) -> Result<Diagnostics, MathError> {
    decode_with_header(bytes)
}

/// Encode a feasibility classification with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if the list length or any violated index
/// exceeds the `u32` wire-format range.
pub fn encode_feasibility(feasibility: &Feasibility) -> Result<Vec<u8>, MathError> {
    encode_with_header(feasibility)
}

/// Decode a feasibility classification produced by [`encode_feasibility`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_feasibility(bytes: &[u8]) -> Result<Feasibility, MathError> {
    decode_with_header(bytes)
}

/// Encode a slice of constraint violations with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if the slice length exceeds the `u32` wire-format range.
pub fn encode_constraint_violations(
    violations: &[ConstraintViolation],
) -> Result<Vec<u8>, MathError> {
    encode_slice_with_header(violations)
}

/// Decode a slice of constraint violations produced by [`encode_constraint_violations`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_constraint_violations(bytes: &[u8]) -> Result<Vec<ConstraintViolation>, MathError> {
    decode_slice_with_header(bytes)
}

/// Encode an evaluation outcome with the schema-version header.
///
/// # Errors
/// [`ErrorKind::IndexOutOfRange`] if any embedded payload or the error message
/// exceeds the `u32` wire-format range.
pub fn encode_evaluation_outcome(outcome: &EvaluationOutcome) -> Result<Vec<u8>, MathError> {
    encode_with_header(outcome)
}

/// Decode an evaluation outcome produced by [`encode_evaluation_outcome`].
///
/// # Errors
/// See the module documentation for the full set of decode failures.
pub fn decode_evaluation_outcome(bytes: &[u8]) -> Result<EvaluationOutcome, MathError> {
    decode_with_header(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_u32_boundaries() {
        assert_eq!(checked_u32(u32::MAX as usize, "value").unwrap(), u32::MAX);
        let error = checked_u32(u32::MAX as usize + 1, "value").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::IndexOutOfRange);
    }
}
