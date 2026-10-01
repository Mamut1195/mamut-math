//! Ask/tell session tests: generation loop, stop-reason mapping, objective-count
//! discovery/validation, abort and evaluation-failure handling.

use mamut_math_core::{
    Candidate, Diagnostics, ErrorKind, EvaluationOutcome, Feasibility, MathError, ObjectiveValues,
    Seed, StopReason, ValidFloat, Variable, VariableId, VariableKind,
};
use mamut_nsga3::{Nsga3Config, Nsga3Session};

fn continuous_variable() -> Vec<Variable> {
    vec![Variable::new(
        VariableId(0),
        VariableKind::continuous(ValidFloat::new(0.0).unwrap(), ValidFloat::new(1.0).unwrap())
            .unwrap(),
    )
    .unwrap()]
}

fn config(population_size: usize, generations: u32) -> Nsga3Config {
    Nsga3Config::new(
        population_size,
        generations,
        0.9,
        0.1,
        5.0,
        50.0,
        4,
        Seed::from_u64(42),
        continuous_variable(),
    )
    .unwrap()
}

/// A stub evaluator producing `m` objectives derived from the candidate.
fn evaluator(m: usize) -> impl FnMut(&[Candidate]) -> Vec<EvaluationOutcome> {
    move |candidates: &[Candidate]| {
        candidates
            .iter()
            .map(|candidate| {
                let x = candidate.as_slice()[0].get();
                let objectives = (0..m)
                    .map(|k| ValidFloat::new(x + k as f64).unwrap())
                    .collect();
                EvaluationOutcome::Success {
                    objectives: ObjectiveValues::new(objectives).unwrap(),
                    constraints: Vec::new(),
                    feasibility: Feasibility::Feasible,
                    diagnostics: Diagnostics::new(Vec::new()),
                }
            })
            .collect()
    }
}

/// A configuration with an explicit seed (for deterministic scenario search).
fn config_seeded(population_size: usize, generations: u32, seed: u64) -> Nsga3Config {
    Nsga3Config::new(
        population_size,
        generations,
        0.9,
        0.1,
        5.0,
        50.0,
        4,
        Seed::from_u64(seed),
        continuous_variable(),
    )
    .unwrap()
}

/// An evaluator that marks `x < threshold` hard-infeasible and the rest feasible.
fn feasibility_evaluator(threshold: f64) -> impl FnMut(&[Candidate]) -> Vec<EvaluationOutcome> {
    move |candidates: &[Candidate]| {
        candidates
            .iter()
            .map(|candidate| {
                let x = candidate.as_slice()[0].get();
                let objectives = ObjectiveValues::new(vec![
                    ValidFloat::new(x).unwrap(),
                    ValidFloat::new(1.0 - x).unwrap(),
                ])
                .unwrap();
                let feasibility = if x >= threshold {
                    Feasibility::Feasible
                } else {
                    Feasibility::InfeasibleHard { violated: vec![0] }
                };
                EvaluationOutcome::Success {
                    objectives,
                    constraints: Vec::new(),
                    feasibility,
                    diagnostics: Diagnostics::new(Vec::new()),
                }
            })
            .collect()
    }
}

#[test]
fn new_session_asks_for_the_initial_population() {
    let session = Nsga3Session::new(config(6, 3)).unwrap();
    let pending = session.ask();
    assert_eq!(pending.len(), 6);
    assert_eq!(session.generation(), 0);
    assert_eq!(session.evaluations(), 0);
    assert!(session.stop_reason().is_none());
    assert!(session.objectives().is_none());
}

#[test]
fn session_runs_to_the_generation_budget() {
    let mut session = Nsga3Session::new(config(8, 2)).unwrap();
    let mut evaluate = evaluator(3);
    let mut progress = session.step(&mut evaluate).unwrap();
    for _ in 0..16 {
        if progress.stop_reason.is_some() {
            break;
        }
        progress = session.step(&mut evaluate).unwrap();
    }
    assert_eq!(
        progress.stop_reason,
        Some(StopReason::IterationBudgetExhausted)
    );
    assert_eq!(session.objectives(), Some(3));
    assert!(session.evaluations() >= 8);
    // Selected indices always point into a valid pool.
    assert!(session.selected_indices().iter().all(|&i| i < 16));
}

#[test]
fn evaluate_only_run_with_zero_generations_stops_immediately() {
    let mut session = Nsga3Session::new(config(4, 0)).unwrap();
    let mut evaluate = evaluator(2);
    let progress = session.step(&mut evaluate).unwrap();
    assert_eq!(
        progress.stop_reason,
        Some(StopReason::IterationBudgetExhausted)
    );
    assert_eq!(session.evaluations(), 4);
}

#[test]
fn tell_requires_matching_outcome_count() {
    let mut session = Nsga3Session::new(config(5, 2)).unwrap();
    assert_eq!(session.ask().len(), 5);
    let error = session.tell(vec![]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DimensionMismatch);
}

#[test]
fn evaluation_failure_is_recorded_and_stops_the_session() {
    let mut session = Nsga3Session::new(config(4, 2)).unwrap();
    let pending = session.ask();
    let mut outcomes = evaluator(2)(&pending);
    outcomes[0] = EvaluationOutcome::Failure {
        error: MathError::new(ErrorKind::Reserved, "evaluator exploded"),
    };
    session.tell(outcomes).unwrap();
    assert_eq!(session.stop_reason(), Some(StopReason::Failed));
    assert_eq!(session.failure().unwrap().message(), "evaluator exploded");
}

#[test]
fn objective_arity_change_is_rejected() {
    let mut session = Nsga3Session::new(config(4, 2)).unwrap();
    let mut evaluate_two = evaluator(2);
    session.step(&mut evaluate_two).unwrap();
    assert_eq!(session.objectives(), Some(2));

    // The next generation is evaluated with a different arity.
    let mut evaluate_three = evaluator(3);
    let error = session.step(&mut evaluate_three).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DimensionMismatch);
}

#[test]
fn abort_stops_and_reports_aborted() {
    let mut session = Nsga3Session::new(config(4, 2)).unwrap();
    session.abort();
    assert_eq!(session.stop_reason(), Some(StopReason::Aborted));
    let mut evaluate = evaluator(2);
    // Further steps are no-ops and do not evaluate.
    let progress = session.step(&mut evaluate).unwrap();
    assert_eq!(progress.stop_reason, Some(StopReason::Aborted));
    assert_eq!(session.evaluations(), 0);
}

#[test]
fn same_seed_produces_the_same_final_selection() {
    let run = |seed: u64| {
        let cfg = Nsga3Config::new(
            8,
            2,
            0.9,
            0.1,
            5.0,
            50.0,
            4,
            Seed::from_u64(seed),
            continuous_variable(),
        )
        .unwrap();
        let mut session = Nsga3Session::new(cfg).unwrap();
        let mut evaluate = evaluator(3);
        let mut progress = session.step(&mut evaluate).unwrap();
        for _ in 0..16 {
            if progress.stop_reason.is_some() {
                break;
            }
            progress = session.step(&mut evaluate).unwrap();
        }
        session.selected_indices().to_vec()
    };
    assert_eq!(run(42), run(42));
}

/// BLOCKER 1(a): with a shortage of admissible candidates, survivors are drawn
/// only from admissible individuals; no `InfeasibleHard` index is retained.
#[test]
fn shortage_of_admissible_candidates_never_retains_infeasible_hard() {
    // Deterministically locate a seed whose initial population is mixed: at
    // least 2 admissible (feasible) and at least 1 hard-infeasible, with the
    // admissible count below the population size (2 <= admissible < 8).
    let mut chosen = None;
    for seed in 0..2000u64 {
        let session = Nsga3Session::new(config_seeded(8, 2, seed)).unwrap();
        let initial = session.ask();
        let admissible: Vec<usize> = initial
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.as_slice()[0].get() >= 0.5)
            .map(|(index, _)| index)
            .collect();
        if (2..8).contains(&admissible.len()) {
            chosen = Some((session, admissible));
            break;
        }
    }
    let (mut session, admissible) = chosen.expect("a mixed-population seed exists");

    let mut evaluate = feasibility_evaluator(0.5);
    session.step(&mut evaluate).unwrap();

    // The session continued (>= 2 admissible) and kept exactly the admissible
    // individuals as survivors — no hard-infeasible backfill.
    assert_eq!(session.selected_indices().len(), admissible.len());
    for &index in session.selected_indices() {
        assert!(
            admissible.contains(&index),
            "infeasible-hard survivor {index} was retained"
        );
    }
    assert_eq!(session.generation(), 1);
    assert!(session.stop_reason().is_none());
}

/// BLOCKER 1(b): an all-hard-infeasible pool terminates with a typed failure,
/// never a normal completion.
#[test]
fn all_hard_infeasible_pool_terminates_with_typed_failure() {
    let mut session = Nsga3Session::new(config(4, 2)).unwrap();
    // `x >= f64::INFINITY` is never true, so every candidate is InfeasibleHard.
    let mut evaluate = feasibility_evaluator(f64::INFINITY);

    let error = session.step(&mut evaluate).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::EmptyCollection);
    assert_eq!(session.stop_reason(), Some(StopReason::Failed));
    assert!(session.failure().is_some());
    assert_ne!(session.stop_reason(), Some(StopReason::Converged));
    // No next generation was produced.
    assert_eq!(session.generation(), 0);
}
