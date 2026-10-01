//! Explicit terminal state for a run.
//!
//! [`StopReason`] is the single terminal-state field of a manifest: there is no
//! separate status enum. Failure, abort and unknown states deliberately carry
//! **no** message so that no secret or domain payload can leak through them.

use crate::codec::{write_u64, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};

/// Why a run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StopReason {
    /// The algorithm reached its convergence criterion.
    Converged,
    /// The run reached its configured target.
    TargetAchieved,
    /// The evaluation budget was exhausted.
    EvaluationBudgetExhausted,
    /// The iteration budget was exhausted.
    IterationBudgetExhausted,
    /// A logical deadline was reached after `ticks` logical ticks.
    LogicalDeadlineReached {
        /// The logical tick count at which the deadline fired.
        ticks: u64,
    },
    /// The run was aborted by the caller.
    Aborted,
    /// The run failed.
    Failed,
    /// The terminal state is not known.
    Unknown,
}

impl StopReason {
    /// Whether this is a normal completion (tags 0–4).
    #[must_use]
    pub fn is_normal_completion(&self) -> bool {
        matches!(
            self,
            StopReason::Converged
                | StopReason::TargetAchieved
                | StopReason::EvaluationBudgetExhausted
                | StopReason::IterationBudgetExhausted
                | StopReason::LogicalDeadlineReached { .. }
        )
    }

    /// Whether this is an abnormal termination (tags 5–7).
    #[must_use]
    pub fn is_abnormal_termination(&self) -> bool {
        matches!(
            self,
            StopReason::Aborted | StopReason::Failed | StopReason::Unknown
        )
    }
}

impl Canonical for StopReason {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            StopReason::Converged => out.push(0),
            StopReason::TargetAchieved => out.push(1),
            StopReason::EvaluationBudgetExhausted => out.push(2),
            StopReason::IterationBudgetExhausted => out.push(3),
            StopReason::LogicalDeadlineReached { ticks } => {
                out.push(4);
                write_u64(*ticks, out);
            }
            StopReason::Aborted => out.push(5),
            StopReason::Failed => out.push(6),
            StopReason::Unknown => out.push(7),
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(StopReason::Converged),
            1 => Ok(StopReason::TargetAchieved),
            2 => Ok(StopReason::EvaluationBudgetExhausted),
            3 => Ok(StopReason::IterationBudgetExhausted),
            4 => Ok(StopReason::LogicalDeadlineReached {
                ticks: dec.read_u64()?,
            }),
            5 => Ok(StopReason::Aborted),
            6 => Ok(StopReason::Failed),
            7 => Ok(StopReason::Unknown),
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown StopReason tag",
            )),
        }
    }
}
