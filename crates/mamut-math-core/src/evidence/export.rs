//! Backend-neutral canonical JSON export for manifests.

use core::fmt::Write as _;

use crate::error::MathError;
use crate::evaluation::DiagnosticCode;
use crate::value::ValidFloat;

use super::config::ConfigValue;
use super::manifest::RunManifest;
use super::solver::RunKind;
use super::stop::StopReason;

/// Export a value to MAMUT canonical JSON.
///
/// The `Result` return is part of the port's uniform signature so that future
/// exporters (which may allocate or fail) share one interface. The
/// [`RunManifest`] implementation is infallible by construction.
pub trait EvidenceExport {
    /// Serialize this value to MAMUT canonical JSON.
    ///
    /// # Errors
    /// Implementations may fail; the [`RunManifest`] implementation never does.
    fn export(&self) -> Result<String, MathError>;
}

impl EvidenceExport for RunManifest {
    fn export(&self) -> Result<String, MathError> {
        let mut out = String::new();
        out.push('{');

        let _ = write!(out, "\"manifest_version\":{}", self.manifest_version());

        out.push_str(",\"run_id\":");
        let _ = write!(out, "\"{}\"", self.run_id().value());

        out.push_str(",\"fingerprint\":{\"algorithm\":");
        push_string(&mut out, self.fingerprint().algorithm());
        out.push_str(",\"digest\":\"");
        push_hex_digest(&mut out, self.fingerprint().digest());
        out.push_str("\"}");

        out.push_str(",\"solver_versions\":[");
        for (index, solver) in self.solver_versions().iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":");
            push_string(&mut out, solver.id());
            out.push_str(",\"version\":");
            push_string(&mut out, solver.version());
            out.push('}');
        }
        out.push(']');

        out.push_str(",\"run_kind\":");
        match self.run_kind() {
            RunKind::Deterministic => out.push_str("{\"kind\":\"deterministic\"}"),
            RunKind::Stochastic { seed, rng } => {
                out.push_str("{\"kind\":\"stochastic\",\"seed\":");
                let _ = write!(out, "\"{seed}\"");
                out.push_str(",\"rng_algorithm\":");
                push_string(&mut out, rng.algorithm());
                out.push_str(",\"rng_version\":");
                push_string(&mut out, rng.version());
                out.push('}');
            }
        }

        out.push_str(",\"params\":[");
        for (index, param) in self.params().iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            push_string(&mut out, param.name());
            out.push_str(",\"value\":");
            match param.value() {
                ConfigValue::Float(value) => {
                    out.push_str("{\"type\":\"float\",\"value\":");
                    push_float(&mut out, *value);
                    out.push('}');
                }
                ConfigValue::Integer(value) => {
                    let _ = write!(out, "{{\"type\":\"integer\",\"value\":{value}}}");
                }
                ConfigValue::Text(value) => {
                    out.push_str("{\"type\":\"text\",\"value\":");
                    push_string(&mut out, value);
                    out.push('}');
                }
                ConfigValue::Logical(value) => {
                    let _ = write!(out, "{{\"type\":\"logical\",\"value\":{value}}}");
                }
            }
            out.push('}');
        }
        out.push(']');

        out.push_str(",\"precision\":{\"absolute_tolerance\":");
        push_float(&mut out, self.precision().absolute_tolerance());
        out.push_str(",\"relative_tolerance\":");
        push_float(&mut out, self.precision().relative_tolerance());
        out.push('}');

        out.push_str(",\"target\":");
        match self.target() {
            None => out.push_str("null"),
            Some(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    push_float(&mut out, *value);
                }
                out.push(']');
            }
        }

        let evaluation = self.evaluation();
        let _ = write!(
            out,
            ",\"evaluation\":{{\"completed_evaluations\":{},\"failed_evaluations\":{},\
             \"logical_ticks_used\":{}",
            evaluation.completed_evaluations(),
            evaluation.failed_evaluations(),
            evaluation.logical_ticks_used()
        );
        out.push_str(",\"logical_tick_budget\":");
        match evaluation.logical_tick_budget() {
            None => out.push_str("null"),
            Some(budget) => {
                let _ = write!(out, "{budget}");
            }
        }
        out.push('}');

        out.push_str(",\"stop_reason\":");
        match self.stop_reason() {
            StopReason::Converged => out.push_str("{\"reason\":\"converged\"}"),
            StopReason::TargetAchieved => out.push_str("{\"reason\":\"target_achieved\"}"),
            StopReason::EvaluationBudgetExhausted => {
                out.push_str("{\"reason\":\"evaluation_budget_exhausted\"}");
            }
            StopReason::IterationBudgetExhausted => {
                out.push_str("{\"reason\":\"iteration_budget_exhausted\"}");
            }
            StopReason::LogicalDeadlineReached { ticks } => {
                let _ = write!(
                    out,
                    "{{\"reason\":\"logical_deadline_reached\",\"ticks\":{ticks}}}"
                );
            }
            StopReason::Aborted => out.push_str("{\"reason\":\"aborted\"}"),
            StopReason::Failed => out.push_str("{\"reason\":\"failed\"}"),
            StopReason::Unknown => out.push_str("{\"reason\":\"unknown\"}"),
        }

        out.push_str(",\"diagnostics\":[");
        for (index, diagnostic) in self.diagnostics().iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"code\":");
            push_string(&mut out, diagnostic_code_label(diagnostic.code));
            out.push_str(",\"note\":");
            push_string(&mut out, &diagnostic.note);
            out.push('}');
        }
        out.push(']');

        out.push_str(",\"artifacts\":[");
        for (index, artifact) in self.artifacts().iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            push_string(&mut out, artifact.name());
            out.push_str(",\"algorithm\":");
            push_string(&mut out, artifact.algorithm());
            out.push_str(",\"digest\":\"");
            push_hex_digest(&mut out, artifact.digest());
            out.push_str("\"}");
        }
        out.push(']');

        out.push('}');
        Ok(out)
    }
}

/// Append a JSON string literal with MAMUT escaping.
///
/// Escapes `"` and `\`, and every code point in `U+0000..=U+001F` as a
/// four-digit `\u00xx` sequence (lowercase hex, no short forms such as `\n`).
/// Every other code point is emitted literally; `/` is never escaped.
fn push_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            control if (control as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Append a quoted `0x`-prefixed 16-digit lowercase hex float encoding.
fn push_float(out: &mut String, value: ValidFloat) {
    let _ = write!(out, "\"0x{:016x}\"", value.get().to_bits());
}

/// Append 32 digest bytes as 64 lowercase hex digits (unquoted).
fn push_hex_digest(out: &mut String, digest: &[u8; 32]) {
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
}

/// The canonical JSON label of a diagnostic code.
fn diagnostic_code_label(code: DiagnosticCode) -> &'static str {
    match code {
        DiagnosticCode::DegenerateFront => "degenerate_front",
        DiagnosticCode::EvaluationWarning => "evaluation_warning",
        DiagnosticCode::Reserved => "reserved",
    }
}
