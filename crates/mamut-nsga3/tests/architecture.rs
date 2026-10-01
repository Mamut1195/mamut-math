//! Architecture tests: dependency isolation, no third-party types in the
//! public API, and `f64` canonical signatures (mirrors
//! `mamut-math-core/tests/architecture.rs`, extraction contract §A4).

use mamut_math_core::rng::MamutRng;
use mamut_math_core::{
    Candidate, EvaluationOutcome, Feasibility, MathError, ObjectiveValues, Seed, ValidFloat,
    Variable, VariableId, VariableKind,
};
use mamut_nsga3::{
    adaptive_eta_value, associate, fast_nondominated_sort, init_population, normalize,
    polynomial_mutation, sbx_crossover, select_environmental, uniform_reference_points, var_or,
    Individual, NormalizeResult, Nsga3Config, Nsga3Session,
};

/// Collect the dependency keys declared under a TOML section.
fn section_keys(manifest: &str, section: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut in_section = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == section;
            continue;
        }
        if in_section && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some((key, _)) = trimmed.split_once('=') {
                keys.push(key.trim().to_string());
            }
        }
    }
    keys
}

/// The only declared dependency must be `mamut-math-core`; no `[dev-dependencies]`.
#[test]
fn no_third_party_dependencies() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest must be readable");

    assert_eq!(
        section_keys(&manifest, "[dependencies]"),
        vec!["mamut-math-core".to_string()],
        "the only [dependencies] entry must be mamut-math-core:\n{manifest}"
    );
    assert!(
        section_keys(&manifest, "[dev-dependencies]").is_empty(),
        "no [dev-dependencies] table is allowed (parity fixtures are hand-written):\n{manifest}"
    );
}

/// Drive the entire public API with core types only, invoking no solver and
/// crossing no third-party boundary.
#[test]
fn public_api_composes_with_an_app_model() {
    let refs = uniform_reference_points(5, 4).unwrap();

    // A caller-owned fitness matrix, one 5-objective row per individual.
    let fitnesses: Vec<Vec<f64>> = (0..12)
        .map(|i| {
            let t = i as f64 / 12.0;
            vec![
                1.0 + t,
                1.0 + (1.0 - t),
                1.0 + 0.5 * t,
                1.0 + 0.5 * (1.0 - t),
                1.0,
            ]
        })
        .collect();

    let fronts = fast_nondominated_sort(&fitnesses).unwrap();
    assert!(!fronts.is_empty());

    let normalized = normalize(&fitnesses).unwrap();
    let assoc = associate(&normalized.normalized, &refs).unwrap();
    assert_eq!(assoc.len(), fitnesses.len());

    let mut rng = MamutRng::root(&Seed::from_u64(0));
    let selected = select_environmental(&fitnesses, 6, &refs, &mut rng).unwrap();
    assert_eq!(selected.len(), 6);

    // Genetic operators over caller-supplied boxes.
    let variables = vec![Variable::new(
        VariableId(0),
        VariableKind::continuous(ValidFloat::new(0.0).unwrap(), ValidFloat::new(1.0).unwrap())
            .unwrap(),
    )
    .unwrap()];
    let mut population = init_population(&variables, 4, &mut rng).unwrap();
    assert_eq!(population.len(), 4);
    let mut child = population.remove(0);
    let mut other = population.pop().unwrap();
    sbx_crossover(&mut child, &mut other, &variables, 15.0, &mut rng).unwrap();
    polynomial_mutation(&mut child, &variables, 20.0, &mut rng).unwrap();
    let offspring = var_or(
        &[child.clone(), other.clone()],
        &variables,
        3,
        0.7,
        0.2,
        15.0,
        &mut rng,
    )
    .unwrap();
    assert_eq!(offspring.len(), 3);
    assert!(adaptive_eta_value(0, 10, 5.0, 50.0) >= 5.0);

    // Ask/tell session over a stub evaluator that returns core types only.
    let config =
        Nsga3Config::new(6, 2, 0.9, 0.1, 5.0, 50.0, 4, Seed::from_u64(7), variables).unwrap();
    let mut session = Nsga3Session::new(config).unwrap();
    let mut evaluate = |candidates: &[Candidate]| -> Vec<EvaluationOutcome> {
        candidates
            .iter()
            .map(|candidate| {
                let x = candidate.as_slice()[0].get();
                let objectives = ObjectiveValues::new(vec![
                    ValidFloat::new(x).unwrap(),
                    ValidFloat::new(1.0 - x).unwrap(),
                ])
                .unwrap();
                EvaluationOutcome::Success {
                    objectives,
                    constraints: Vec::new(),
                    feasibility: Feasibility::Feasible,
                    diagnostics: mamut_math_core::Diagnostics::new(Vec::new()),
                }
            })
            .collect()
    };
    let mut progress = session.step(&mut evaluate).unwrap();
    for _ in 0..8 {
        if progress.stop_reason.is_some() {
            break;
        }
        progress = session.step(&mut evaluate).unwrap();
    }
    assert!(progress.stop_reason.is_some());
    let _: Individual = offspring[0].clone();
}

/// Every public numeric kernel and type is expressed with `f64`/`usize`/core
/// MAMUT types — there is no generic scalar parameter.
#[test]
#[allow(clippy::type_complexity)]
fn f64_canonical_signatures() {
    let _: fn(usize, usize) -> Result<Vec<Vec<f64>>, MathError> = uniform_reference_points;
    let _: fn(&[Vec<f64>]) -> Result<Vec<Vec<usize>>, MathError> = fast_nondominated_sort;
    let _: fn(&[Vec<f64>]) -> Result<NormalizeResult, MathError> = normalize;
    let _: fn(&[f64], &[f64]) -> Result<f64, MathError> = mamut_nsga3::perp_distance;
    let _: fn(u32, u32, f64, f64) -> f64 = adaptive_eta_value;
    // `Individual` carries core `ObjectiveValues`, never a raw third-party type.
    fn assert_no_generic<T>() {}
    assert_no_generic::<Individual>();
}
