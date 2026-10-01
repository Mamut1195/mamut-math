"""Shared helpers for the offline NSGA-III conformance oracles.

OFFLINE TEST TOOLING ONLY. This module is never imported by production Rust;
it exists to generate committed JSON fixtures and their provenance. It is
intentionally dependency-light: numpy + the installed pymoo/deap.

All reference values produced here trace to one of:
  * an independent library (pymoo NSGA3 / DEAP operators and primitives), or
  * a closed-form literature Pareto front sampled in-script.

Nothing in this file calls the MAMUT Rust crate, so Rust-vs-fixture parity
cannot be circular.
"""

from __future__ import annotations

import datetime
import json
import os
import sys

import numpy as np


# ---------------------------------------------------------------------------
# Provenance
# ---------------------------------------------------------------------------


def module_versions() -> dict:
    """Return the exact toolchain versions recorded in every fixture."""
    import numpy
    import pymoo
    import deap

    versions = {
        "python": sys.version.split()[0],
        "python_executable": sys.executable,
        "numpy": numpy.__version__,
        "pymoo": pymoo.__version__,
        "deap": deap.__version__,
        "deap_nsga3_available": False,
        "deap_nsga3_note": (
            "DEAP 1.4 ships no NSGA-III algorithm. pymoo NSGA3 is the "
            "algorithm-level oracle; DEAP supplies component-level oracles "
            "(sortNondominated, exact 3D hypervolume, cxSimulatedBinaryBounded, "
            "mutPolynomialBounded)."
        ),
    }
    try:
        import mpmath

        versions["mpmath"] = mpmath.__version__
    except Exception:  # pragma: no cover - mpmath is present in the oracle env
        versions["mpmath"] = None
    return versions


def provenance(script_name: str) -> dict:
    """Build the provenance block stamped into a fixture."""
    return {
        "generated_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "script": os.path.basename(script_name),
        "module_versions": module_versions(),
        "evidence_class": "independent-oracle",
        "mamut_crate_used": False,
    }


def write_json(path: str, payload: dict) -> None:
    """Write a fixture as UTF-8 JSON with stable key order and indent."""
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


# ---------------------------------------------------------------------------
# Exact hypervolume for 3 objectives (independent of the Rust slicer)
# ---------------------------------------------------------------------------


def _nondominated(points):
    """Return the Pareto-optimal (minimization) subset of ``points``."""
    import numpy as np

    pts = np.asarray(points, dtype=float)
    if pts.size == 0:
        return pts.reshape(0, 3)
    keep = np.ones(len(pts), dtype=bool)
    for i in range(len(pts)):
        # i is dominated iff some j is <= in all and < in at least one.
        dominated = np.all(pts <= pts[i], axis=1) & np.any(pts < pts[i], axis=1)
        if dominated.any():
            keep[i] = False
    return pts[keep]


def hv3d(points, ref) -> float:
    """Exact 3-objective dominated hypervolume (minimization).

    Iterative objective-sweep exact algorithm: sweep the first objective into
    slabs and accumulate the exact 2D union area of the ``(y, z)`` projections
    per slab, with each 2D area computed by an exact 1D union sweep. This is a
    structurally different implementation from the recursive slicer inside
    ``mamut-nsga3`` (``hypervolume_slices``), so agreement is real evidence and
    not a shared implementation.

    Points are expected in minimization orientation; points not strictly inside
    the reference box contribute zero volume.
    """
    import numpy as np

    pts = np.asarray(points, dtype=float)
    ref = np.asarray(ref, dtype=float)
    if pts.size == 0:
        return 0.0
    pts = pts[np.all(pts < ref, axis=1)]
    if len(pts) == 0:
        return 0.0
    pts = _nondominated(pts)

    # Remove exact duplicates so the level sets are clean.
    pts = np.unique(pts, axis=0)

    xs = sorted(set(pts[:, 0].tolist()) | {float(ref[0])})
    volume = 0.0
    for i in range(len(xs) - 1):
        x_lo, x_hi = xs[i], xs[i + 1]
        if x_hi <= x_lo:
            continue
        active = pts[pts[:, 0] <= x_lo]
        area = _union_area_2d(active[:, 1], active[:, 2], ref[1], ref[2])
        volume += area * (x_hi - x_lo)
    return float(volume)


def _union_area_2d(ys, zs, ref_y, ref_z) -> float:
    """Exact area of the union of ``[y, ref_y] x [z, ref_z]`` rectangles."""
    if len(ys) == 0:
        return 0.0
    ys = list(ys)
    zs = list(zs)
    levels = sorted(set(ys) | {ref_y})
    area = 0.0
    for i in range(len(levels) - 1):
        y_lo, y_hi = levels[i], levels[i + 1]
        if y_hi <= y_lo:
            continue
        # Each rectangle is [y_i, ref_y] x [z_i, ref_z]; for y in this strip the
        # active rectangles are those with y_i <= y_lo, and their z-union covers
        # [min z_i, ref_z].
        z_min = None
        for y, z in zip(ys, zs):
            if y <= y_lo:
                if z_min is None or z < z_min:
                    z_min = z
        if z_min is None:
            continue
        height = ref_z - z_min
        if height > 0.0:
            area += height * (y_hi - y_lo)
    return float(area)


def hv_bruteforce(points, ref) -> float:
    """Inclusion-exclusion hypervolume (exponential; small-N self-check only)."""
    from itertools import combinations

    pts = [tuple(float(v) for v in p) for p in points]
    ref = tuple(float(v) for v in ref)
    pts = [p for p in pts if all(p[j] < ref[j] for j in range(len(ref)))]
    total = 0.0
    for r in range(1, len(pts) + 1):
        for subset in combinations(pts, r):
            box = [max(p[j] for p in subset) for j in range(len(ref))]
            vol = 1.0
            for j in range(len(ref)):
                vol *= max(ref[j] - box[j], 0.0)
            total += ((-1.0) ** (r + 1)) * vol
    return float(total)


# ---------------------------------------------------------------------------
# IGD+ (Ishibuchi, Masuda, Tanigaki & Nojima 2015)
# ---------------------------------------------------------------------------


def igd_plus(front, reference_front) -> float:
    """IGD+ of ``front`` against a reference subset of the true PF.

    ``d+(a, r) = sqrt(sum_j max(a_j - r_j, 0)^2)``; only axes where the
    approximation ``a`` is worse (larger, minimization) than the reference
    ``r`` contribute.
    """
    import numpy as np

    front = np.asarray(front, dtype=float)
    ref = np.asarray(reference_front, dtype=float)
    if front.size == 0 or ref.size == 0:
        raise ValueError("igd_plus requires non-empty fronts")
    total = 0.0
    for r in ref:
        diff = front - r
        clipped = np.maximum(diff, 0.0)
        dists = np.sqrt(np.sum(clipped * clipped, axis=1))
        total += float(dists.min())
    return total / len(ref)


# ---------------------------------------------------------------------------
# Literature Pareto fronts (DTLZ, Deb & Thiele 1999 / Deb 2001)
# ---------------------------------------------------------------------------


def _das_dennis_weights(m: int, p: int):
    """Uniform simplex lattice (independent transcription)."""
    import numpy as np

    result = []
    current = [0] * m

    def rec(depth: int, remaining: int):
        if depth == m - 1:
            current[depth] = remaining
            result.append([v / p for v in current])
            return
        for value in range(remaining + 1):
            current[depth] = value
            rec(depth + 1, remaining - value)

    rec(0, p)
    return np.asarray(result, dtype=float)


def pf_dtlz1(n_obj: int = 3, p: int = 45):
    """DTLZ1 PF: the simplex ``{f >= 0 : sum f = 0.5}`` (Deb/Thiele 1999).

    Nondominance is automatic: every sampled point has ``sum f = 0.5`` exactly,
    so if ``p <= q`` coordinate-wise then ``sum p <= sum q``; equality of the
    two sums forces ``p == q``. Hence no sampled point strictly dominates
    another and the full simplex sampling is already the nondominated front.
    """
    weights = _das_dennis_weights(n_obj, p)
    return 0.5 * weights


def pf_dtlz2(n_obj: int = 3, p: int = 45):
    """DTLZ2 PF: the positive unit-sphere patch (Deb/Thiele 1999).

    Nondominance is automatic: every sampled point is normalised to unit norm,
    so if ``p <= q`` coordinate-wise with ``p != q`` then ``||p|| < ||q||``,
    contradicting the equal unit norms. Hence the points are mutually
    nondominated and no filter is required.
    """
    import numpy as np

    weights = _das_dennis_weights(n_obj, p)
    norms = np.linalg.norm(weights, axis=1, keepdims=True)
    norms[norms == 0.0] = 1.0
    return weights / norms


# Human-supplied corroboration constants for the DTLZ7 record-high endpoints
# (NOT the derivation source; the values are derived numerically below and the
# constants are asserted against the derivation to <= 1e-14).
DTLZ7_ENDPOINT_CORROBORATION = (
    0.251411836088917,  # a: first local maximum of A
    0.631626530700061,  # b: later record crossing A(b) = A(a)
    0.859400856644724,  # c: global maximum of A
)


def _dtlz7_A(x):
    return x * (1.0 + np.sin(3.0 * np.pi * x))


def derive_dtlz7_endpoints():
    """Derive the DTLZ7 record-high endpoints ``(a, b, c)`` at 50 digits.

    ``A(x) = x (1 + sin(3 pi x))`` on ``[0, 1]``. The record-high set of ``A``
    is ``[0, a] U [b, c]`` where

    * ``a`` is the first local maximum: the smallest positive root of
      ``A'(x) = 1 + sin(3 pi x) + 3 pi x cos(3 pi x) = 0`` with ``A'' < 0``;
    * ``b`` is the later crossing of the record, i.e. the root of
      ``A(x) = A(a)`` after the interior minimum;
    * ``c`` is the global maximum: the larger-``A`` root of ``A'(x) = 0`` in
      ``[0.75, 1]``.

    Roots are found by bracketed ``mpmath.findroot`` (secant) at 50 decimal
    digits, then cast to ``f64``. The human constants in
    ``DTLZ7_ENDPOINT_CORROBORATION`` are asserted to agree within ``1e-14``.
    """
    import mpmath as mp

    mp.mp.dps = 50

    def a_fun(x):
        return x * (1 + mp.sin(3 * mp.pi * x))

    def a_prime(x):
        return 1 + mp.sin(3 * mp.pi * x) + 3 * mp.pi * x * mp.cos(3 * mp.pi * x)

    a = mp.findroot(a_prime, (0.24, 0.26))
    c = mp.findroot(a_prime, (0.84, 0.88))
    b = mp.findroot(lambda x: a_fun(x) - a_fun(a), (0.6, 0.65))
    derived = (float(a), float(b), float(c))

    for name, value, expected in zip("abc", derived, DTLZ7_ENDPOINT_CORROBORATION):
        assert abs(value - expected) <= 1e-14, (
            f"DTLZ7 endpoint {name}={value!r} differs from corroboration "
            f"{expected!r} by more than 1e-14"
        )
    return derived


def pf_dtlz7_grid_raw(n_obj: int = 3, grid: int = 32):
    """Legacy raw DTLZ7 grid (no nondominance filter); regression demonstration.

    Kept ONLY to demonstrate that the naive grid wrongly keeps dominated samples
    (e.g. the raw sample ``[0, 8/31, ...]``). Do not use as a reference front.
    """
    axis = [i / (grid - 1) for i in range(grid)]
    rows = []
    for f1 in axis:
        for f2 in axis:
            prefix = [f1, f2][: n_obj - 1]
            g = 1.0
            h = n_obj - sum(
                f / (1.0 + g) * (1.0 + np.sin(3.0 * np.pi * f)) for f in prefix
            )
            rows.append(prefix + [(1.0 + g) * h])
    return np.asarray(rows, dtype=float)


def pf_dtlz7_certified(n_obj: int = 3, n_per_interval: int = 20):
    """MATHEMATICALLY CERTIFIED DTLZ7 Pareto front (``M = 3``, ``g = 1``).

    On the DTLZ7 PF (``g = 1``, ``x_3..x_n = 0.5``) the surface is
    ``f_3 = 6 - A(f_1) - A(f_2)`` with ``A(x) = x (1 + sin(3 pi x))`` and
    ``f_1, f_2 in [0, 1]``. A surface point ``(x, y, f_3)`` is nondominated on
    the continuous front IFF ``x`` and ``y`` are RECORD-HIGH points of ``A``,
    i.e. ``A(t) >= A(u)`` for all ``u <= t``. The record-high set is
    ``[0, a] U [b, c]`` (derived in :func:`derive_dtlz7_endpoints`), so points
    with ``f_1`` or ``f_2`` in the excluded gap ``(a, b)`` are continuously
    dominated and are NOT part of the true front.

    Refinement at the fold (derived): ``A`` is strictly increasing on ``[0, a]``
    and on ``[b, c]``, and the two branches meet only at the single value
    ``A(a) = A(b)``. At that tie the smaller coordinate is better for
    minimization (same ``f_3``, smaller ``f_1``), so the point at ``b`` is
    dominated by the point at ``a``. The certified Pareto coordinate set is
    therefore ``[0, a] U (b, c]`` — the ENDPOINT ``b`` is excluded (the high
    interval is sampled open at ``b``); ``c`` is a global maximum and is kept.

    The front is the union of the ``2^(M-1) = 4`` Cartesian patches over the
    certified coordinate intervals, each sampled uniformly with
    ``n_per_interval`` points, so the reference has ``4 * n_per_interval^2``
    points.

    References: Deb & Thiele 1999 (DTLZ7 disconnected PF); Ishibuchi et al. 2015
    (IGD+ reference-front contract).
    """
    if n_obj != 3:
        raise ValueError("pf_dtlz7_certified is defined for M = 3")
    a, b, c = derive_dtlz7_endpoints()

    def samples(lo, hi, drop_left):
        # n uniform samples; drop_left excludes the folded endpoint (open at lo).
        xs = np.linspace(lo, hi, n_per_interval + (1 if drop_left else 0))
        return xs[1:] if drop_left else xs

    # (low, high, drop_left): [0,a] closed; (b,c] open at b (fold tie removed).
    intervals = [(0.0, a, False), (b, c, True)]
    rows = []
    for lo1, hi1, dl1 in intervals:
        for lo2, hi2, dl2 in intervals:
            for f1 in samples(lo1, hi1, dl1):
                for f2 in samples(lo2, hi2, dl2):
                    rows.append([f1, f2, 6.0 - _dtlz7_A(f1) - _dtlz7_A(f2)])
    return np.asarray(rows, dtype=float)


def pf_dtlz7(n_obj: int = 3, n_per_interval: int = 20):
    """Certified DTLZ7 reference front (see :func:`pf_dtlz7_certified`)."""
    return pf_dtlz7_certified(n_obj, n_per_interval)


PF_BUILDERS = {
    "dtlz1": pf_dtlz1,
    "dtlz2": pf_dtlz2,
    "dtlz7": pf_dtlz7,
}

# Fixed HV reference points for the 3-objective DTLZ instances (minimization).
HV_REFERENCE = {
    "dtlz1": [0.6, 0.6, 0.6],
    "dtlz2": [1.1, 1.1, 1.1],
    "dtlz7": [1.1, 1.1, 6.5],
}
