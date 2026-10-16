"""Deterministic pseudo-random data shared by the Python and Rust harnesses.

numpy's generators cannot be reproduced bit-for-bit outside numpy, so figure
cases draw "random" data from this tiny SplitMix64-based generator instead.
plt-spec implements the identical algorithm in Rust; keep the two in sync.
"""

from __future__ import annotations

import math

import numpy as np

_MASK = (1 << 64) - 1


class SpecRng:
    """SplitMix64 stream with uniform, normal and integer helpers."""

    def __init__(self, seed: int) -> None:
        self._state = seed & _MASK

    def next_u64(self) -> int:
        self._state = (self._state + 0x9E3779B97F4A7C15) & _MASK
        z = self._state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & _MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & _MASK
        return z ^ (z >> 31)

    def next_f64(self) -> float:
        """Uniform in [0, 1) with 53 bits of precision."""
        return (self.next_u64() >> 11) * (1.0 / (1 << 53))

    def uniform(self, n: int, low: float = 0.0, high: float = 1.0) -> np.ndarray:
        return np.array([low + (high - low) * self.next_f64() for _ in range(n)])

    def normal(self, n: int, loc: float = 0.0, scale: float = 1.0) -> np.ndarray:
        """Box–Muller; each sample consumes two uniforms (no caching)."""
        out = np.empty(n)
        for i in range(n):
            u1 = 1.0 - self.next_f64()  # (0, 1]: safe for log
            u2 = self.next_f64()
            out[i] = loc + scale * math.sqrt(-2.0 * math.log(u1)) * math.cos(2.0 * math.pi * u2)
        return out

    def integers(self, n: int, low: int, high: int) -> np.ndarray:
        """Integers in [low, high) via multiply-shift (no modulo bias concerns at spec sizes)."""
        span = high - low
        return np.array([low + ((self.next_u64() >> 11) * span >> 53) for _ in range(n)], dtype=np.int64)


def rng(seed: int) -> SpecRng:
    return SpecRng(seed)
