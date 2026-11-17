# Check that matplotlib's warnings point at this file (warn_external): filename and line number of each recorded warning.
import os
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def record(fn):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fn()
    return [(os.path.basename(w.filename), w.lineno, w.category.__name__) for w in caught]


fig, ax = plt.subplots()
print(record(lambda: ax.set_xlim(2, 2)))
print(record(lambda: ax.legend()))
print(record(lambda: ax.plot([1, 2], "r", color="g")))


def helper_library_call():
    return ax.set_ylim(3, 3)


print(record(helper_library_call))
print(record(lambda: plt.subplots()[1].set_yticklabels(["a"])))
print(record(lambda: ax.set_xscale("log") or ax.set_xlim(-2, 5)))


def tight_with_free_axes():
    f = plt.figure()
    f.add_axes([0, 0, 1, 1])
    f.tight_layout()


print(record(tight_with_free_axes))
print(record(lambda: plt.figure().add_subplot().pcolormesh(np.zeros((2, 2)), shading="gouraud", edgecolors="k")))
