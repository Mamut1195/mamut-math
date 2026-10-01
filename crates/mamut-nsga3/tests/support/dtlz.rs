//! DTLZ1 / DTLZ2 / DTLZ7 objectives transcribed from the literature.
//!
//! Sources: Deb, Thiele, Laumanns & Zitzler, "Scalable multi-objective
//! optimization test problems" (DTLZ, 2002) and Deb, "Multi-Objective
//! Optimization using Evolutionary Algorithms" (DTLZ7, 2001). The 3-objective
//! instances use the standard `k` (DTLZ1 k=5 -> n=7, DTLZ2 k=10 -> n=12,
//! DTLZ7 k=20 -> n=22). These in-test transcriptions were cross-checked against
//! pymoo's independent implementations at ~1e-13 absolute difference (ulp-level
//! relative error) over random vectors (see `pymoo_dtlz_oracle.json` provenance).

/// A 3-objective test problem with an evaluator over `[0, 1]^n_var`.
pub struct Problem {
    pub name: &'static str,
    pub n_var: usize,
    pub evaluate: fn(&[f64]) -> Vec<f64>,
}

/// DTLZ1 (linear PF on the simplex `sum f = 0.5`), M=3, k=5, n=7.
///
/// `g = 100 (k + sum_{i=M..n} [(x_i - 0.5)^2 - cos(20 pi (x_i - 0.5))])`;
/// `f_1 = 0.5(1+g) x_1 x_2`, `f_2 = 0.5(1+g) x_1 (1-x_2)`,
/// `f_3 = 0.5(1+g)(1-x_1)`.
pub fn dtlz1(x: &[f64]) -> Vec<f64> {
    let m = 3usize;
    let n = x.len();
    let k = n - m + 1;
    let mut g = k as f64;
    for &xi in &x[m - 1..] {
        let d = xi - 0.5;
        g += d * d - (20.0 * std::f64::consts::PI * d).cos();
    }
    let g = 100.0 * g;
    let base = 0.5 * (1.0 + g);
    let f0 = base * x[0] * x[1];
    let f1 = base * x[0] * (1.0 - x[1]);
    let f2 = base * (1.0 - x[0]);
    vec![f0, f1, f2]
}

/// DTLZ2 (spherical PF on the positive unit sphere), M=3, k=10, n=12.
///
/// `g = sum_{i=M..n} (x_i - 0.5)^2`;
/// `f_1 = (1+g) cos(x_1 pi/2) cos(x_2 pi/2)`,
/// `f_2 = (1+g) cos(x_1 pi/2) sin(x_2 pi/2)`, `f_3 = (1+g) sin(x_1 pi/2)`.
pub fn dtlz2(x: &[f64]) -> Vec<f64> {
    let m = 3usize;
    let mut g = 0.0;
    for &xi in &x[m - 1..] {
        let d = xi - 0.5;
        g += d * d;
    }
    let half_pi = std::f64::consts::FRAC_PI_2;
    let a = (x[0] * half_pi).cos();
    let b = (x[0] * half_pi).sin();
    let c = (x[1] * half_pi).cos();
    let d = (x[1] * half_pi).sin();
    vec![(1.0 + g) * a * c, (1.0 + g) * a * d, (1.0 + g) * b]
}

/// DTLZ7 (four disconnected PF patches), M=3, k=20, n=22.
///
/// `g = 1 + 9/k sum_{i=M..n} x_i`; `f_1 = x_1`, `f_2 = x_2`,
/// `h = M - sum_{i=1}^{M-1} f_i/(1+g) (1 + sin(3 pi f_i))`,
/// `f_3 = (1+g) h`.
pub fn dtlz7(x: &[f64]) -> Vec<f64> {
    let m = 3usize;
    let n = x.len();
    let k = n - m + 1;
    let mut sum_tail = 0.0;
    for &xi in &x[m - 1..] {
        sum_tail += xi;
    }
    let g = 1.0 + 9.0 / k as f64 * sum_tail;
    let f1 = x[0];
    let f2 = x[1];
    let h = m as f64
        - (f1 / (1.0 + g) * (1.0 + (3.0 * std::f64::consts::PI * f1).sin())
            + f2 / (1.0 + g) * (1.0 + (3.0 * std::f64::consts::PI * f2).sin()));
    vec![f1, f2, (1.0 + g) * h]
}

/// Build a named 3-objective problem (`"dtlz1"`, `"dtlz2"`, `"dtlz7"`).
pub fn problem(name: &str) -> Problem {
    match name {
        "dtlz1" => Problem {
            name: "dtlz1",
            n_var: 7,
            evaluate: dtlz1,
        },
        "dtlz2" => Problem {
            name: "dtlz2",
            n_var: 12,
            evaluate: dtlz2,
        },
        "dtlz7" => Problem {
            name: "dtlz7",
            n_var: 22,
            evaluate: dtlz7,
        },
        other => panic!("unknown problem `{other}`"),
    }
}
