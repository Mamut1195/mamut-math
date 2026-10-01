"""Algorithm-level conformance oracle (offline): pymoo NSGA3 on DTLZ1/2/7.

Independent algorithm oracle for the MAMUT NSGA-III quality bands. Runs pymoo
NSGA3 with das-dennis p=12 reference directions (91 points), population 92 and
an exact 9200-evaluation budget over seeds 0..7 for the 3-objective DTLZ1,
DTLZ2 and DTLZ7 problems. Hypervolume and IGD+ are computed IN-SCRIPT from
closed-form literature Pareto fronts (not from pymoo internals), using the
independent exact 3D hypervolume in ``oracle_common``.

Run:
    python scripts/oracles/pymoo_full_oracle.py

Output:
    crates/mamut-nsga3/tests/fixtures/conformance/pymoo_dtlz_oracle.json
"""

from __future__ import annotations

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import oracle_common as oc  # noqa: E402

from pymoo.algorithms.moo.nsga3 import NSGA3  # noqa: E402
from pymoo.optimize import minimize  # noqa: E402
from pymoo.problems.many.dtlz import DTLZ1, DTLZ2, DTLZ7  # noqa: E402
from pymoo.util.ref_dirs import get_reference_directions  # noqa: E402

FIXTURE = os.path.join(
    os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
    "crates",
    "mamut-nsga3",
    "tests",
    "fixtures",
    "conformance",
    "pymoo_dtlz_oracle.json",
)

N_OBJ = 3
DIVISIONS = 12
POP_SIZE = 92
EVAL_BUDGET = 9200
SEEDS = list(range(8))
ROUND_DP = 14

# (name, pymoo class, literature n_var = k + M - 1)
PROBLEMS = [
    ("dtlz1", DTLZ1, 7),
    ("dtlz2", DTLZ2, 12),
    ("dtlz7", DTLZ7, 22),
]


def nondominated_front(f: np.ndarray) -> np.ndarray:
    """Nondominated (minimization) subset preserving order."""
    pts = np.asarray(f, dtype=float)
    keep = np.ones(len(pts), dtype=bool)
    for i in range(len(pts)):
        dominated = np.all(pts <= pts[i], axis=1) & np.any(pts < pts[i], axis=1)
        if dominated.any():
            keep[i] = False
    return pts[keep]


def rounded(matrix, dp=ROUND_DP):
    return [[round(float(v), dp) for v in row] for row in np.asarray(matrix)]


def compare_pymoo_dtlz7(problem, a, b, c):
    """SECONDARY comparison of the certified DTLZ7 region against pymoo's PF.

    Reports the fraction of pymoo PF points whose ``(f1, f2)`` lie inside the
    certified record-high region ``[0,a] U [b,c]`` (per coordinate) and the
    coordinate ranges. This corroborates patch coverage/ranges; it is NOT used
    as the membership proof (that comes from the record-high derivation).
    """
    pf = np.asarray(problem.pareto_front(), dtype=float)
    tol = 1e-9

    def in_region(v):
        return (v <= a + tol) | ((v >= b - tol) & (v <= c + tol))

    mask = in_region(pf[:, 0]) & in_region(pf[:, 1])
    return {
        "pymoo_pf_size": int(pf.shape[0]),
        "fraction_in_certified_region": float(mask.mean()),
        "pymoo_ranges": {
            "f1": [float(pf[:, 0].min()), float(pf[:, 0].max())],
            "f2": [float(pf[:, 1].min()), float(pf[:, 1].max())],
            "f3": [float(pf[:, 2].min()), float(pf[:, 2].max())],
        },
        "note": "secondary corroboration only; membership proven by the record-high derivation",
    }


def main() -> int:
    ref_dirs = get_reference_directions("das-dennis", N_OBJ, n_partitions=DIVISIONS)
    assert ref_dirs.shape == (91, 3), ref_dirs.shape

    problems_payload = {}
    for name, cls, n_var in PROBLEMS:
        builder = oc.PF_BUILDERS[name]
        pf = builder(N_OBJ)
        hv_ref = oc.HV_REFERENCE[name]
        pymoo_problem = cls(n_var=n_var, n_obj=N_OBJ)
        assert pymoo_problem.n_var == n_var

        runs = []
        for seed in SEEDS:
            res = minimize(
                pymoo_problem,
                NSGA3(ref_dirs, pop_size=POP_SIZE),
                ("n_evals", EVAL_BUDGET),
                seed=seed,
                verbose=False,
            )
            f = np.asarray(res.pop.get("F"), dtype=float)
            n_eval = int(res.algorithm.evaluator.n_eval)
            front = nondominated_front(f)
            hv = oc.hv3d(front, hv_ref)
            igd = oc.igd_plus(front, pf)
            runs.append(
                {
                    "seed": seed,
                    "n_evals": n_eval,
                    "population_size": int(f.shape[0]),
                    "front_size": int(front.shape[0]),
                    "hv": hv,
                    "igd_plus": igd,
                    "final_front": rounded(front),
                    "final_population": rounded(f),
                }
            )
            print(f"{name} seed={seed} n_evals={n_eval} hv={hv:.6f} igd+={igd:.6f}")

        extra = {}
        if name == "dtlz7":
            a, b, c = oc.derive_dtlz7_endpoints()
            extra["certified_intervals"] = {
                "a": a,
                "b": b,
                "c": c,
                "record_high_set": "[0,a] U [b,c]",
                "pareto_coordinate_set": "[0,a] U (b,c] (endpoint b excluded: fold tie A(a)=A(b))",
                "n_per_interval": 20,
                "construction": (
                    "4 Cartesian patches over certified coordinate intervals; "
                    "f3 = 6 - A(f1) - A(f2), A(x) = x(1+sin(3*pi*x)); the high "
                    "interval is open at b because A(a)=A(b) and the smaller "
                    "coordinate a dominates b"
                ),
            }
            extra["pymoo_pf_comparison"] = compare_pymoo_dtlz7(
                pymoo_problem, a, b, c
            )

        problems_payload[name] = {
            "n_var": n_var,
            "n_obj": N_OBJ,
            "k": n_var - N_OBJ + 1,
            "hv_reference_point": hv_ref,
            "pf_size": int(pf.shape[0]),
            "pf_points": rounded(pf),
            "runs": runs,
            **extra,
        }

    payload = {
        "schema_version": 1,
        "fixture_name": "pymoo_nsga3_dtlz_conformance",
        "provenance": oc.provenance(__file__),
        "algorithm": "pymoo.utils NSGA3 (Deb & Jain 2014 reference-direction survival)",
        "reference_directions": {
            "method": "das-dennis",
            "n_obj": N_OBJ,
            "divisions": DIVISIONS,
            "count": int(ref_dirs.shape[0]),
        },
        "population_size": POP_SIZE,
        "eval_budget": EVAL_BUDGET,
        "seeds": SEEDS,
        "problem_defs": {
            name: {
                "class": cls.__name__,
                "n_var": n_var,
                "n_obj": N_OBJ,
                "formula_source": "Deb & Thiele 1999 / Deb 2001 (transcribed to Rust in conformance_quality.rs)",
            }
            for name, cls, n_var in PROBLEMS
        },
        "metric_definitions": {
            "hypervolume": (
                "exact 3-objective dominated hypervolume (minimization) computed "
                "in-script (oracle_common.hv3d)"
            ),
            "igd_plus": (
                "Ishibuchi et al. 2015 IGD+ against a literature PF sampled "
                "in-script (oracle_common.igd_plus)"
            ),
        },
        "problems": problems_payload,
    }
    oc.write_json(FIXTURE, payload)
    print(f"wrote {FIXTURE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
