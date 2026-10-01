//! Pareto-metric parity tests: analytical cases, the 5-objective DEAP /
//! independent-Python golden, and the frozen `metrics.rs` unit-test
//! expectations ported to the dynamic-`M` API.
//!
//! Frozen provenance (read-only):
//! `git -C <hydraulics-repo> show 6db4862d7a4035de8d494cdc62607b12b4edb103:hydro-optimizer/src/metrics.rs`
//! plus its committed fixture
//! `tests/oracle/fixtures/optimizer_quality_metrics_golden.json` (re-encoded
//! into `tests/fixtures/metrics_5d_golden.json`).
//!
//! Tolerances: `f64` parity at `1e-12` absolute (the frozen gate precedent);
//! identity/degenerate assertions are exact (`== 0.0`).

use mamut_math_core::ErrorKind;
use mamut_nsga3::{denormalize_from_unit_box, hypervolume, igd_plus, normalize_to_unit_box};

/// Minimal dependency-free JSON reader for the committed golden fixture.
///
/// The architecture test forbids any `[dev-dependencies]`, so this parses only
/// the subset the fixture uses: objects, arrays, numbers and strings.
mod json {
    #[derive(Debug)]
    pub enum Json {
        Object(Vec<(String, Json)>),
        Array(Vec<Json>),
        Number(f64),
        String(String),
    }

    impl Json {
        pub fn field(&self, key: &str) -> &Json {
            match self {
                Json::Object(entries) => entries
                    .iter()
                    .find(|(entry, _)| entry == key)
                    .map(|(_, value)| value)
                    .unwrap_or_else(|| panic!("missing JSON field `{key}`")),
                _ => panic!("expected JSON object to read field `{key}`"),
            }
        }

        pub fn number(&self) -> f64 {
            match self {
                Json::Number(value) => *value,
                other => panic!("expected JSON number, found {other:?}"),
            }
        }

        pub fn text(&self) -> &str {
            match self {
                Json::String(value) => value,
                other => panic!("expected JSON string, found {other:?}"),
            }
        }

        pub fn vector(&self) -> Vec<f64> {
            match self {
                Json::Array(items) => items.iter().map(Json::number).collect(),
                other => panic!("expected JSON array, found {other:?}"),
            }
        }

        pub fn matrix(&self) -> Vec<Vec<f64>> {
            match self {
                Json::Array(items) => items.iter().map(Json::vector).collect(),
                other => panic!("expected JSON matrix, found {other:?}"),
            }
        }
    }

    pub fn parse(input: &str) -> Result<Json, String> {
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        parser.skip_ws();
        let value = parser.value()?;
        parser.skip_ws();
        if parser.pos != parser.bytes.len() {
            return Err(format!("trailing data at byte {}", parser.pos));
        }
        Ok(value)
    }

    struct Parser<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl Parser<'_> {
        fn skip_ws(&mut self) {
            while let Some(byte) = self.peek() {
                if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.pos).copied()
        }

        fn value(&mut self) -> Result<Json, String> {
            self.skip_ws();
            match self.peek() {
                Some(b'{') => self.object(),
                Some(b'[') => self.array(),
                Some(b'"') => Ok(Json::String(self.string()?)),
                Some(_) => self.number(),
                None => Err("unexpected end of JSON input".to_string()),
            }
        }

        fn object(&mut self) -> Result<Json, String> {
            self.pos += 1; // consume '{'
            let mut entries = Vec::new();
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(Json::Object(entries));
            }
            loop {
                self.skip_ws();
                let key = self.string()?;
                self.skip_ws();
                if self.peek() != Some(b':') {
                    return Err(format!("expected ':' at byte {}", self.pos));
                }
                self.pos += 1;
                let value = self.value()?;
                entries.push((key, value));
                self.skip_ws();
                match self.peek() {
                    Some(b',') => self.pos += 1,
                    Some(b'}') => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(format!("expected ',' or '}}' at byte {}", self.pos)),
                }
            }
            Ok(Json::Object(entries))
        }

        fn array(&mut self) -> Result<Json, String> {
            self.pos += 1; // consume '['
            let mut items = Vec::new();
            self.skip_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(Json::Array(items));
            }
            loop {
                let value = self.value()?;
                items.push(value);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => self.pos += 1,
                    Some(b']') => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(format!("expected ',' or ']' at byte {}", self.pos)),
                }
            }
            Ok(Json::Array(items))
        }

        fn string(&mut self) -> Result<String, String> {
            if self.peek() != Some(b'"') {
                return Err(format!("expected '\"' at byte {}", self.pos));
            }
            self.pos += 1;
            let mut out = String::new();
            while let Some(byte) = self.peek() {
                self.pos += 1;
                match byte {
                    b'"' => return Ok(out),
                    b'\\' => {
                        let escaped = self.peek().ok_or("unterminated escape")?;
                        self.pos += 1;
                        out.push(match escaped {
                            b'"' => '"',
                            b'\\' => '\\',
                            b'/' => '/',
                            b'n' => '\n',
                            b't' => '\t',
                            b'r' => '\r',
                            other => other as char,
                        });
                    }
                    other => out.push(other as char),
                }
            }
            Err("unterminated JSON string".to_string())
        }

        fn number(&mut self) -> Result<Json, String> {
            let start = self.pos;
            if self.peek() == Some(b'-') {
                self.pos += 1;
            }
            while let Some(byte) = self.peek() {
                if byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-') {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let text = std::str::from_utf8(&self.bytes[start..self.pos])
                .map_err(|_| "invalid UTF-8 in JSON number".to_string())?;
            text.parse::<f64>()
                .map(Json::Number)
                .map_err(|_| format!("invalid JSON number `{text}`"))
        }
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected}, got {actual} (|delta| = {})",
        (actual - expected).abs()
    );
}

// ───────────────────────── analytical hypervolume ─────────────────────────

#[test]
fn hv_2d_unit_square_is_exact() {
    // Box [1,2] x [1,2] -> area 1.
    let value = hypervolume(&[vec![1.0, 1.0]], &[2.0, 2.0]).unwrap();
    assert_close(value, 1.0, 1e-12, "2D unit square");
}

#[test]
fn hv_2d_l_shape_is_exact() {
    // Points (1,3) and (3,1) with reference (4,4): hand-computed union area 5.
    let value = hypervolume(&[vec![1.0, 3.0], vec![3.0, 1.0]], &[4.0, 4.0]).unwrap();
    assert_close(value, 5.0, 1e-12, "2D L-shape");
}

#[test]
fn hv_3d_single_point_cube_is_exact() {
    // Box [1,3]^3 -> 2^3 = 8.
    let value = hypervolume(&[vec![1.0, 1.0, 1.0]], &[3.0, 3.0, 3.0]).unwrap();
    assert_close(value, 8.0, 1e-12, "3D single cube");
}

#[test]
fn hv_3d_two_point_union_is_exact() {
    // Frozen `test_hv_two_points_3d_union`: union volume 6.
    let front = vec![vec![1.0, 1.0, 2.0], vec![2.0, 1.0, 1.0]];
    let value = hypervolume(&front, &[3.0, 3.0, 3.0]).unwrap();
    assert_close(value, 6.0, 1e-12, "3D two-point union");
}

#[test]
fn hv_5d_single_point_is_exact() {
    // Frozen `test_hv_single_point_5d`: [1,3]^5 -> 2^5 = 32.
    let value = hypervolume(&[vec![1.0; 5]], &[3.0; 5]).unwrap();
    assert_close(value, 32.0, 1e-12, "5D single point");
}

#[test]
fn hv_empty_front_is_exact_zero() {
    // The empty set has measure exactly 0, not an error and not a placeholder.
    assert_eq!(hypervolume(&[], &[2.0, 2.0]).unwrap(), 0.0);
}

#[test]
fn hv_duplicates_and_dominated_points_are_measure_neutral() {
    let reference = [3.0, 3.0];
    let single = hypervolume(&[vec![1.0, 1.0]], &reference).unwrap();
    assert_close(single, 4.0, 1e-12, "single box");
    let noisy = hypervolume(
        &[
            vec![1.0, 1.0],
            vec![1.0, 1.0], // duplicate
            vec![1.5, 1.2], // dominated by [1,1]
            vec![2.0, 2.0], // dominated by [1,1]
        ],
        &reference,
    )
    .unwrap();
    assert_close(noisy, single, 1e-12, "duplicate/dominated neutrality");
}

#[test]
fn hv_points_outside_reference_box_contribute_zero() {
    let reference = [2.0, 2.0];
    // [1,1] contributes [1,2]^2 = 1; [3,3] is beyond the box on both axes.
    let value = hypervolume(&[vec![1.0, 1.0], vec![3.0, 3.0]], &reference).unwrap();
    assert_close(value, 1.0, 1e-12, "outside-box neutrality");
    assert_eq!(hypervolume(&[vec![5.0, 5.0]], &reference).unwrap(), 0.0);
}

#[test]
fn hv_reference_validation_errors() {
    assert_eq!(
        hypervolume(&[vec![1.0, 1.0]], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        hypervolume(&[], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        hypervolume(&[vec![1.0, 1.0]], &[2.0, f64::NAN])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
}

#[test]
fn hv_dimension_mismatch_errors() {
    // Front arity 3 vs reference arity 2.
    assert_eq!(
        hypervolume(&[vec![1.0, 1.0, 1.0]], &[2.0, 2.0])
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
    // Ragged front.
    assert_eq!(
        hypervolume(&[vec![1.0, 1.0], vec![1.0]], &[2.0, 2.0])
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
}

#[test]
fn hv_non_finite_front_is_invalid_not_finite() {
    assert_eq!(
        hypervolume(&[vec![1.0, f64::INFINITY]], &[2.0, 2.0])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
}

// ───────────────────────── IGD+ ─────────────────────────

#[test]
fn igd_plus_identity_is_exactly_zero() {
    let front = vec![
        vec![0.0, 2.0, 1.0],
        vec![1.0, 1.0, 2.0],
        vec![2.0, 0.0, 0.5],
    ];
    assert_eq!(igd_plus(&front, &front).unwrap(), 0.0);
}

#[test]
fn igd_plus_reference_better_than_front_is_sqrt2() {
    // Frozen `test_igdplus_reference_better_than_front`.
    let value = igd_plus(&[vec![1.0, 1.0]], &[vec![0.0, 0.0]]).unwrap();
    assert_close(value, 2.0_f64.sqrt(), 1e-12, "IGD+ reference-better");
}

#[test]
fn igd_plus_front_better_than_reference_is_zero() {
    // Frozen `test_igdplus_front_better_than_reference_is_zero`: worse axes clip.
    assert_eq!(igd_plus(&[vec![0.0, 0.0]], &[vec![1.0, 1.0]]).unwrap(), 0.0);
}

#[test]
fn igd_plus_empty_inputs_are_empty_collection() {
    // Hardened frozen behavior: the `∞` placeholder is replaced by a typed error.
    assert_eq!(
        igd_plus(&[], &[vec![1.0, 1.0]]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        igd_plus(&[vec![1.0, 1.0]], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection
    );
}

#[test]
fn igd_plus_validation_errors() {
    assert_eq!(
        igd_plus(&[vec![1.0, 2.0]], &[vec![1.0]])
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
    assert_eq!(
        igd_plus(&[vec![f64::NAN, 1.0]], &[vec![1.0, 1.0]])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    assert_eq!(
        igd_plus(&[vec![1.0, 1.0]], &[vec![1.0, f64::INFINITY]])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    // Ragged reference front.
    assert_eq!(
        igd_plus(&[vec![1.0, 1.0]], &[vec![1.0, 1.0], vec![1.0]])
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
}

// ───────────────────────── unit-box normalization ─────────────────────────

#[test]
fn normalize_to_unit_box_maps_known_points() {
    let front = vec![vec![0.0, 0.0], vec![2.0, 4.0], vec![1.0, 2.0]];
    let normed = normalize_to_unit_box(&front, &[0.0, 0.0], &[2.0, 4.0]).unwrap();
    assert_eq!(normed, vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![0.5, 0.5]]);
}

#[test]
fn normalize_to_unit_box_handles_nonzero_lower_bound() {
    let front = vec![vec![1.0, -1.0], vec![3.0, 1.0]];
    let normed = normalize_to_unit_box(&front, &[1.0, -1.0], &[3.0, 1.0]).unwrap();
    assert_eq!(normed, vec![vec![0.0, 0.0], vec![1.0, 1.0]]);
}

#[test]
fn normalize_denormalize_round_trip_within_tolerance() {
    let front = vec![
        vec![0.3, 12.0, -4.0],
        vec![2.7, 18.5, 1.5],
        vec![1.1, 15.25, -0.25],
    ];
    let lower = [0.3, 12.0, -4.0];
    let upper = [3.0, 20.0, 2.0];
    let normed = normalize_to_unit_box(&front, &lower, &upper).unwrap();
    for row in &normed {
        for &value in row {
            assert!((0.0..=1.0).contains(&value));
        }
    }
    let restored = denormalize_from_unit_box(&normed, &lower, &upper).unwrap();
    for (original, back) in front.iter().zip(restored.iter()) {
        for (&a, &b) in original.iter().zip(back.iter()) {
            assert_close(b, a, 1e-12, "round-trip");
        }
    }
}

#[test]
fn normalize_rejects_constant_objective_as_domain_mismatch() {
    // Zero-width range on axis 1 -> typed error, never a silent NaN/Inf.
    assert_eq!(
        normalize_to_unit_box(&[vec![1.0, 1.0]], &[0.0, 1.0], &[2.0, 1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::DomainMismatch
    );
    assert_eq!(
        denormalize_from_unit_box(&[vec![0.5, 0.5]], &[0.0, 1.0], &[2.0, 1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::DomainMismatch
    );
}

#[test]
fn normalize_rejects_inverted_bounds() {
    assert_eq!(
        normalize_to_unit_box(&[vec![1.0]], &[3.0], &[1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::InvertedBounds
    );
}

#[test]
fn normalize_validates_empty_arity_and_finiteness() {
    assert_eq!(
        normalize_to_unit_box(&[], &[0.0], &[1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(
        normalize_to_unit_box(&[vec![1.0, 2.0]], &[0.0], &[1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
    assert_eq!(
        normalize_to_unit_box(&[vec![f64::NAN, 1.0]], &[0.0, 0.0], &[1.0, 1.0])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    assert_eq!(
        normalize_to_unit_box(&[vec![1.0, 1.0]], &[0.0, 0.0], &[1.0, f64::INFINITY])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
}

// ───────────────────────── frozen unit-test parity ─────────────────────────

#[test]
fn frozen_metrics_unit_cases_are_reproduced() {
    // Ported from the frozen `metrics.rs` unit tests (`git show 6db4862d…`).
    // Hypervolume cases (fully compatible with the dynamic-M API):
    assert_close(
        hypervolume(&[vec![1.0, 1.0]], &[3.0, 3.0]).unwrap(),
        4.0,
        1e-12,
        "frozen hv 1pt 2d",
    );
    assert_close(
        hypervolume(&[vec![1.0, 3.0], vec![3.0, 1.0]], &[4.0, 4.0]).unwrap(),
        5.0,
        1e-12,
        "frozen hv 2pt 2d",
    );
    assert_close(
        hypervolume(&[vec![1.0; 5]], &[3.0; 5]).unwrap(),
        32.0,
        1e-12,
        "frozen hv 1pt 5d",
    );
    assert_close(
        hypervolume(
            &[vec![1.0, 1.0, 2.0], vec![2.0, 1.0, 1.0]],
            &[3.0, 3.0, 3.0],
        )
        .unwrap(),
        6.0,
        1e-12,
        "frozen hv 2pt 3d union",
    );
    assert_eq!(
        hypervolume(&[], &[2.0, 2.0]).unwrap(),
        0.0,
        "frozen hv empty front"
    );

    // IGD+ cases (compatible):
    assert_eq!(
        igd_plus(
            &[vec![1.0, 2.0], vec![2.0, 1.0]],
            &[vec![1.0, 2.0], vec![2.0, 1.0]],
        )
        .unwrap(),
        0.0,
        "frozen igd perfect match"
    );
    assert_close(
        igd_plus(&[vec![1.0, 1.0]], &[vec![0.0, 0.0]]).unwrap(),
        2.0_f64.sqrt(),
        1e-12,
        "frozen igd reference better",
    );
    assert_eq!(
        igd_plus(&[vec![0.0, 0.0]], &[vec![1.0, 1.0]]).unwrap(),
        0.0,
        "frozen igd front better"
    );

    // Empty-front cases are hardened from the frozen `∞` to typed errors:
    assert_eq!(
        igd_plus(&[], &[vec![1.0, 1.0]]).unwrap_err().kind(),
        ErrorKind::EmptyCollection,
        "frozen igd empty front"
    );
    assert_eq!(
        igd_plus(&[vec![1.0, 1.0]], &[]).unwrap_err().kind(),
        ErrorKind::EmptyCollection,
        "frozen igd empty reference"
    );

    // The 11th frozen case (the DEAP / independent-Python golden) is covered by
    // `five_objective_metrics_match_frozen_oracle` below.
}

// ───────────────────────── 5D golden ─────────────────────────

#[test]
fn five_objective_metrics_match_frozen_oracle() {
    let fixture = json::parse(include_str!("fixtures/metrics_5d_golden.json"))
        .expect("5D golden fixture must be valid JSON");

    assert_eq!(fixture.field("schema_version").number(), 1.0);
    assert_eq!(
        fixture.field("fixture_name").text(),
        "optimizer_quality_metrics_golden"
    );

    let approximation = fixture.field("approximation_front").matrix();
    let reference_front = fixture.field("reference_front").matrix();
    let reference_point = fixture.field("reference_point").vector();
    let oracle_hypervolume = fixture.field("oracle_hypervolume").number();
    let oracle_igd_plus = fixture.field("oracle_igd_plus").number();
    let tolerance = fixture.field("absolute_tolerance").number();

    // Structural pins: 3-point approximation, 4-point reference, 5 objectives.
    assert_eq!(approximation.len(), 3);
    assert_eq!(reference_front.len(), 4);
    assert_eq!(reference_point, vec![6.0; 5]);
    assert_eq!(tolerance, 1e-12);

    let hv = hypervolume(&approximation, &reference_point).unwrap();
    let igd = igd_plus(&approximation, &reference_front).unwrap();

    assert_close(hv, 784.0, tolerance, "5D hypervolume freeze");
    assert_close(igd, 0.6889635872182424, tolerance, "5D IGD+ freeze");
    assert_close(hv, oracle_hypervolume, tolerance, "5D HV vs oracle field");
    assert_close(igd, oracle_igd_plus, tolerance, "5D IGD+ vs oracle field");
}
