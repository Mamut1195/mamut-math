//! Evaluation accounting for a run.

use crate::codec::{write_u64, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};

/// Counters describing how much work a run performed.
///
/// Logical ticks are a deterministic, wall-clock-free measure of effort.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EvaluationRecord {
    completed_evaluations: u64,
    failed_evaluations: u64,
    logical_ticks_used: u64,
    logical_tick_budget: Option<u64>,
}

impl EvaluationRecord {
    /// Build an evaluation record.
    #[must_use]
    pub fn new(
        completed_evaluations: u64,
        failed_evaluations: u64,
        logical_ticks_used: u64,
        logical_tick_budget: Option<u64>,
    ) -> Self {
        Self {
            completed_evaluations,
            failed_evaluations,
            logical_ticks_used,
            logical_tick_budget,
        }
    }

    /// Number of evaluations that completed successfully.
    #[must_use]
    pub fn completed_evaluations(&self) -> u64 {
        self.completed_evaluations
    }

    /// Number of evaluations that failed.
    #[must_use]
    pub fn failed_evaluations(&self) -> u64 {
        self.failed_evaluations
    }

    /// Logical ticks consumed by the run.
    #[must_use]
    pub fn logical_ticks_used(&self) -> u64 {
        self.logical_ticks_used
    }

    /// The logical tick budget, if one was set.
    #[must_use]
    pub fn logical_tick_budget(&self) -> Option<u64> {
        self.logical_tick_budget
    }
}

impl Canonical for EvaluationRecord {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u64(self.completed_evaluations, out);
        write_u64(self.failed_evaluations, out);
        write_u64(self.logical_ticks_used, out);
        match self.logical_tick_budget {
            None => out.push(0),
            Some(budget) => {
                out.push(1);
                write_u64(budget, out);
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let completed_evaluations = dec.read_u64()?;
        let failed_evaluations = dec.read_u64()?;
        let logical_ticks_used = dec.read_u64()?;
        let logical_tick_budget = match dec.read_u8()? {
            0 => None,
            1 => Some(dec.read_u64()?),
            _ => {
                return Err(MathError::new(
                    ErrorKind::Reserved,
                    "unknown logical tick budget option tag",
                ))
            }
        };
        Ok(EvaluationRecord::new(
            completed_evaluations,
            failed_evaluations,
            logical_ticks_used,
            logical_tick_budget,
        ))
    }
}
