"""Seeded sample data for the examples that need noise.

numpy's random generators cannot be reproduced outside numpy, so the
examples draw from SplitMix64, the small generator plt-rs uses, here and in
the Rust examples (``examples/rust/src/lib.rs``) alike: both languages plot
the same points.
"""

import math

import numpy as np

_MASK = (1 << 64) - 1


class Samples:
    """A SplitMix64 stream of uniform and normal draws."""

    def __init__(self, seed):
        self._state = seed & _MASK

    def _next_u64(self):
        self._state = (self._state + 0x9E3779B97F4A7C15) & _MASK
        z = self._state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & _MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & _MASK
        return z ^ (z >> 31)

    def _next_f64(self):
        return (self._next_u64() >> 11) * (1.0 / (1 << 53))

    def uniform(self, n, low=0.0, high=1.0):
        """``n`` draws, uniform in ``[low, high)``."""
        return np.array([low + (high - low) * self._next_f64() for _ in range(n)])

    def normal(self, n, mean=0.0, sd=1.0):
        """``n`` normal draws (Box-Muller, two uniforms per draw)."""
        draws = []
        for _ in range(n):
            u = 1.0 - self._next_f64()
            v = self._next_f64()
            draws.append(mean + sd * math.sqrt(-2.0 * math.log(u)) * math.cos(2.0 * math.pi * v))
        return np.array(draws)
