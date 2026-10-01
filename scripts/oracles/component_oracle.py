"""Component-level conformance oracle (offline).

Feeds hand-seeded fixed matrices through INDEPENDENT implementations and dumps
the results to a committed JSON fixture consumed by the Rust conformance test
``crates/mamut-nsga3/tests/conformance_components.rs``.

Independent oracles used (see provenance.module_versions):
  * das-dennis reference lattices: pymoo ``get_reference_directions``
  * nondominated fronts: DEAP ``tools.sortNondominated`` (Deb 2002)
  * ideal / ASF extreme points: pymoo ``util.normalization.find_ideal`` and
    ``util.normalization.get_extreme_points_c``; intercepts by the same
    Gaussian-elimination formula pymoo uses internally
    (``pymoo/util/normalization.py`` ``get_nadir_point`` lines 269-274)
  * exact 3D hypervolume: DEAP compiled core ``deap.tools._hypervolume.hv``
  * IGD+: in-script numpy per Ishibuchi et al. (2015)
  * SBX + polynomial mutation: DEAP ``cxSimulatedBinaryBounded`` and
    ``mutPolynomialBounded`` driven by an explicit scripted uniform stream

Run:
    python scripts/oracles/component_oracle.py

Output:
    crates/mamut-nsga3/tests/fixtures/conformance/component_oracle.json
"""

from __future__ import annotations

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import oracle_common as oc  # noqa: E402

import deap.base  # noqa: E402
import deap.tools as tools  # noqa: E402
from deap.tools._hypervolume import hv as deap_hv  # noqa: E402
from pymoo.util.normalization import (  # noqa: E402
    find_ideal,
    get_extreme_points_c,
    get_nadir_point,
)
from pymoo.util.ref_dirs import get_reference_directions  # noqa: E402

FIXTURE = os.path.join(
    os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
    "crates",
    "mamut-nsga3",
    "tests",
    "fixtures",
    "conformance",
    "component_oracle.json",
)


# ---------------------------------------------------------------------------
# DEAP minimization individuals
# ---------------------------------------------------------------------------


def _fitness_class(m: int):
    """A DEAP Fitness with minimization weights for ``m`` objectives."""
    return type("FitMin", (deap.base.Fitness,), {"weights": tuple([-1.0] * m)})


class _Ind:
    def __init__(self, genes, fitness):
        self.genes = genes
        self.fitness = fitness


# ---------------------------------------------------------------------------
# Reference lattices
# ---------------------------------------------------------------------------


def reference_lattice_cases():
    cases = []
    for m, p in [(3, 12), (3, 4), (4, 3), (5, 4)]:
        dirs = get_reference_directions("das-dennis", m, n_partitions=p)
        cases.append(
            {
                "name": f"das_dennis_m{m}_p{p}",
                "n_obj": m,
                "divisions": p,
                "count": int(dirs.shape[0]),
                "points": [[float(v) for v in row] for row in dirs],
            }
        )
    return cases


# ---------------------------------------------------------------------------
# Nondominated sort
# ---------------------------------------------------------------------------

# Hand-seeded 12x3 matrix with distinct rows (fronts resolved by DEAP).
ND_MATRIX = [
    [0.10, 0.90, 0.50],
    [0.90, 0.10, 0.50],
    [0.50, 0.50, 0.10],
    [0.30, 0.70, 0.80],
    [0.80, 0.30, 0.70],
    [0.70, 0.80, 0.30],
    [0.20, 0.20, 0.90],
    [0.95, 0.60, 0.60],
    [0.60, 0.95, 0.65],
    [0.65, 0.65, 0.95],
    [0.40, 0.40, 0.40],
    [0.55, 0.45, 0.35],
]


def nondominated_sort_case():
    fit_cls = _fitness_class(3)
    inds = []
    for row in ND_MATRIX:
        fit = fit_cls()
        fit.values = tuple(row)
        inds.append(_Ind(row, fit))

    fronts_of_inds = tools.sortNondominated(inds, len(inds), first_front_only=False)
    id_to_index = {id(ind): i for i, ind in enumerate(inds)}
    fronts = [[id_to_index[id(ind)] for ind in front] for front in fronts_of_inds]

    rank = [0] * len(inds)
    for level, front in enumerate(fronts):
        for index in front:
            rank[index] = level

    return {
        "matrix": ND_MATRIX,
        "fronts": fronts,
        "rank": rank,
        "n_fronts": len(fronts),
    }


# ---------------------------------------------------------------------------
# Normalization (ideal / ASF extreme points / intercepts)
# ---------------------------------------------------------------------------

# Hand-seeded 6x3 matrix. The per-axis minimum is attained (each axis has a
# translated zero), every other translated component is >= 0.1, and the three
# ASF extreme points are distinct (A, B, C below), so:
#   * pymoo's `__F < 1e-3 -> 0` substitution and the Rust untranslated ASF agree
#     exactly, and
#   * the intercept linear system is non-singular.
# A/B/C are the "best in the other two objectives" points that the ASF selects
# per axis; P/Q/R pin each per-axis minimum to 0.
NORMALIZE_MATRIX = [
    [10.0, 0.1, 0.2],  # A: ASF extreme for axis 0
    [0.1, 10.0, 0.3],  # B: ASF extreme for axis 1
    [0.15, 0.25, 10.0],  # C: ASF extreme for axis 2
    [0.0, 4.0, 4.0],  # P: axis-0 minimum
    [4.0, 0.0, 4.0],  # Q: axis-1 minimum
    [4.0, 4.0, 0.0],  # R: axis-2 minimum
]


def normalize_case():
    f = np.asarray(NORMALIZE_MATRIX, dtype=float)
    ideal = find_ideal(f)
    extremes = get_extreme_points_c(f, ideal)

    # Intercept solve, exactly the pymoo `get_nadir_point` formula
    # (pymoo/util/normalization.py lines 269-274): solve M x = 1, intercept = 1/x.
    plane = np.linalg.solve(extremes - ideal, np.ones(f.shape[1]))
    intercepts = 1.0 / plane
    normalized = (f - ideal) / intercepts

    # Cross-check against pymoo's public get_nadir_point (documented caveat:
    # it additionally clips the nadir to the worst observed point).
    worst = f.max(axis=0)
    nadir = get_nadir_point(extremes, ideal, worst, worst, worst)

    return {
        "matrix": NORMALIZE_MATRIX,
        "ideal": [float(v) for v in ideal],
        "extremes": [[float(v) for v in row] for row in extremes],
        "intercepts": [float(v) for v in intercepts],
        "normalized": [[float(v) for v in row] for row in normalized],
        "pymoo_get_nadir_point": [float(v) for v in nadir],
        "intercept_formula": (
            "solve(extremes - ideal, ones) then reciprocal; "
            "pymoo/util/normalization.py get_nadir_point lines 269-274"
        ),
    }


# ---------------------------------------------------------------------------
# Hypervolume (DEAP exact 3D core)
# ---------------------------------------------------------------------------

HV_FRONT = [
    [0.10, 0.60, 0.30],
    [0.20, 0.20, 0.70],
    [0.30, 0.50, 0.20],
    [0.40, 0.30, 0.40],
    [0.15, 0.40, 0.55],
    [0.50, 0.10, 0.25],
]
HV_REF = [1.0, 1.0, 1.0]


def hypervolume_case():
    # Self-check DEAP orientation on a known cube: [1,3]^3 -> 8.
    cube = deap_hv.hypervolume(np.asarray([[1.0, 1.0, 1.0]]), np.asarray([3.0, 3.0, 3.0]))
    assert abs(cube - 8.0) < 1e-12, f"DEAP hv orientation self-check failed: {cube}"

    points = np.asarray(HV_FRONT, dtype=float)
    ref = np.asarray(HV_REF, dtype=float)
    value = float(deap_hv.hypervolume(points, ref))

    # Independent in-script exact HV (must agree).
    independent = oc.hv3d(HV_FRONT, HV_REF)

    return {
        "front": HV_FRONT,
        "reference_point": HV_REF,
        "deap_hypervolume": value,
        "independent_hypervolume": independent,
        "note": "DEAP compiled core `deap.tools._hypervolume.hv.hypervolume` (exact 3D)",
    }


# ---------------------------------------------------------------------------
# IGD+
# ---------------------------------------------------------------------------

IGD_FRONT = [
    [0.10, 0.90, 0.50],
    [0.50, 0.50, 0.10],
    [0.90, 0.10, 0.50],
]
IGD_REFERENCE = [
    [0.00, 1.00, 0.50],
    [0.50, 0.50, 0.00],
    [1.00, 0.00, 0.50],
    [0.33, 0.33, 0.33],
]


def igd_plus_case():
    return {
        "front": IGD_FRONT,
        "reference_front": IGD_REFERENCE,
        "igd_plus": oc.igd_plus(IGD_FRONT, IGD_REFERENCE),
        "formula": "Ishibuchi et al. 2015: mean over r of min_a sqrt(sum max(a-r,0)^2)",
    }


# ---------------------------------------------------------------------------
# SBX + polynomial mutation under a fixed uniform stream
# ---------------------------------------------------------------------------

SBX_BOUNDS = [[0.0, 1.0]] * 5
SBX_P1 = [0.20, 0.35, 0.50, 0.65, 0.80]
SBX_P2 = [0.70, 0.55, 0.90, 0.75, 0.95]
SBX_ETA = 2.0
SBX_OURS_UNIFORMS = [0.25, 0.60, 0.40, 0.15, 0.85]


def _scripted_random(uniforms):
    """Return a drop-in replacement for ``random.random`` over ``uniforms``."""
    state = {"i": 0}

    def _next():
        value = uniforms[state["i"]]
        state["i"] += 1
        return value

    return _next, state


def sbx_case():
    # DEAP consumes (branch, rand, swap) per gene. The stream below routes every
    # gene through the crossover branch (branch <= 0.5) and the
    # `ind1=c1, ind2=c2` swap decision (> 0.5), so DEAP's `rand` sequence is
    # exactly the Rust kernel's one-draw-per-gene stream.
    deap_uniforms = []
    for u in SBX_OURS_UNIFORMS:
        deap_uniforms.extend([0.1, u, 0.9])

    import deap.tools.crossover as crossover_module

    original_random = crossover_module.random.random
    scripted, state = _scripted_random(deap_uniforms)
    crossover_module.random.random = scripted
    try:
        ind1 = list(SBX_P1)
        ind2 = list(SBX_P2)
        tools.cxSimulatedBinaryBounded(ind1, ind2, SBX_ETA, [b[0] for b in SBX_BOUNDS], [b[1] for b in SBX_BOUNDS])
    finally:
        crossover_module.random.random = original_random

    assert state["i"] == len(deap_uniforms), (
        f"DEAP SBX consumed {state['i']} draws, expected {len(deap_uniforms)}"
    )

    return {
        "parent1": SBX_P1,
        "parent2": SBX_P2,
        "bounds": SBX_BOUNDS,
        "eta": SBX_ETA,
        "ours_uniforms": SBX_OURS_UNIFORMS,
        "deap_uniforms": deap_uniforms,
        "deap_child1": [float(v) for v in ind1],
        "deap_child2": [float(v) for v in ind2],
        "stream_note": (
            "Rust sbx_crossover consumes one uniform per differing gene and "
            "assigns deterministically (documented divergence from DEAP, which "
            "consumes branch+rand+swap draws and may swap children). The DEAP "
            "stream here is constructed on the matched path so the *formula* "
            "is compared exactly; ours_uniforms is the effective betaq stream."
        ),
    }


POLY_GENES = [0.20, 0.35, 0.50, 0.65, 0.80]
POLY_BOUNDS = [[0.0, 1.0]] * 5
POLY_ETA = 20.0
POLY_INDPB = 1.0 / 5.0
POLY_UNIFORMS = [0.15, 0.30, 0.80, 0.10, 0.70, 0.90, 0.20, 0.45]


def polynomial_mutation_case():
    import deap.tools.mutation as mutation_module

    original_random = mutation_module.random.random
    scripted, state = _scripted_random(POLY_UNIFORMS)
    mutation_module.random.random = scripted
    try:
        individual = list(POLY_GENES)
        tools.mutPolynomialBounded(
            individual,
            POLY_ETA,
            [b[0] for b in POLY_BOUNDS],
            [b[1] for b in POLY_BOUNDS],
            POLY_INDPB,
        )
    finally:
        mutation_module.random.random = original_random

    assert state["i"] == len(POLY_UNIFORMS), (
        f"DEAP polynomial mutation consumed {state['i']} draws, "
        f"expected {len(POLY_UNIFORMS)}"
    )

    return {
        "genes": POLY_GENES,
        "bounds": POLY_BOUNDS,
        "eta": POLY_ETA,
        "indpb": POLY_INDPB,
        "uniforms": POLY_UNIFORMS,
        "deap_mutated": [float(v) for v in individual],
        "stream_note": (
            "DEAP and the Rust kernel consume the same (indpb-check, rand) "
            "draw sequence; indpb = 1/len is the Rust internal default."
        ),
    }


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> int:
    payload = {
        "schema_version": 1,
        "fixture_name": "nsga3_component_conformance",
        "provenance": oc.provenance(__file__),
        "reference_lattices": reference_lattice_cases(),
        "nondominated_sort": nondominated_sort_case(),
        "normalize": normalize_case(),
        "hypervolume": hypervolume_case(),
        "igd_plus": igd_plus_case(),
        "sbx_crossover": sbx_case(),
        "polynomial_mutation": polynomial_mutation_case(),
    }
    oc.write_json(FIXTURE, payload)
    print(f"wrote {FIXTURE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
