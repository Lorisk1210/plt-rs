"""Data several gallery case modules share: the gallery's favourite fields
and synthetic stand-ins for matplotlib's sample data files.

Figure cases porting matplotlib's gallery examples cannot read the sample
data (``cbook.get_sample_data``); they use arrays of the same shape and
spirit instead. plt-spec builds the same arrays in Rust
(``figures/cases/common.rs``); keep the two in sync.
"""

import math

import numpy as np

from .data import rng

#: The seed of the noise in the stand-ins (the gallery's favourite).
SEED = 19680801

#: libm's ``exp``, value by value. ``numpy.exp`` takes a SIMD path that
#: depends on the CPU and differs from libm in the last bit of some values.
libm_exp = np.vectorize(math.exp)


def two_bumps(X, Y, exp=np.exp):
    """The gallery's favourite field: a positive and a negative bump."""
    Z1 = exp(-X**2 - Y**2)
    Z2 = exp(-(X - 1)**2 - (Y - 1)**2)
    return (Z1 - Z2) * 2


def contour_field():
    """``contour_demo``'s grid: ``two_bumps`` over [-3, 3) x [-2, 2).

    Its zero contour is the straight line x + y = 1, where every label
    position is about as good as any other: which one ``clabel`` picks
    depends on the last bits of the values, so they come from libm's
    ``exp``, which gives the same bits everywhere."""
    X, Y = np.meshgrid(np.arange(-3.0, 3.0, 0.025), np.arange(-2.0, 2.0, 0.025))
    return X, Y, two_bumps(X, Y, exp=libm_exp)


def image_field():
    """``image_demo``'s grid: ``two_bumps`` over [-3, 3)^2."""
    x = y = np.arange(-3.0, 3.0, 0.025)
    X, Y = np.meshgrid(x, y)
    return two_bumps(X, Y)


def scan():
    """A 256 x 256 MRI-like slice in place of ``s1045.ima.gz``: a bright
    ellipse with darker rings inside, values up to 1000."""
    v = np.linspace(-1.0, 1.0, 256)
    X, Y = np.meshgrid(v, v)
    head = np.exp(-((X / 0.75)**2 + (Y / 0.9)**2)**4)
    folds = 0.5 + 0.5 * np.cos(14 * np.hypot(X - 0.1, Y + 0.05))
    return 1000.0 * head * (0.6 + 0.4 * folds)


def terrain():
    """A rough 344 x 403 elevation model in place of
    ``jacksboro_fault_dem.npz``: ridges with noise on top."""
    yy, xx = np.mgrid[0:344, 0:403]
    ridges = 600 + 250 * np.sin(xx / 37.0) * np.cos(yy / 53.0) + 0.5 * (xx - yy)
    noise = rng(SEED).normal(344 * 403, scale=12.0).reshape(344, 403)
    return ridges + noise
