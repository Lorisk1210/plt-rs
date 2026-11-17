# Trigger a catalogue of UserWarnings from ordinary calls and print category, first line and which file the warning points at.
import os
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def check(label, fn):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            fn()
        except Exception as e:
            print(label, "raised", type(e).__name__)
    for w in caught:
        print(label, w.category.__name__, str(w.message).splitlines()[0][:70], os.path.basename(w.filename))
    if not caught:
        print(label, "no warning")


fig, ax = plt.subplots()
check("identical xlim", lambda: ax.set_xlim(1, 1))
check("legend empty", lambda: ax.legend())
check("log nonpositive", lambda: (ax.set_yscale("log"), ax.set_ylim(-1, 10)))
check("ticklabels no locator", lambda: plt.subplots()[1].set_xticklabels(["a", "b"]))
check("fmt and color", lambda: ax.plot([1, 2], "r-", color="b"))
check("marker twice", lambda: ax.plot([1, 2], "o", marker="s"))
check("masked to nan", lambda: float(np.ma.masked_array([1.0], [1])[0]))
check("tight layout", lambda: (lambda f: (f.add_axes([0, 0, 1, 1]), f.tight_layout()))(plt.figure()))
check("scatter c and color", lambda: ax.scatter([1], [1], c="r", color="b"))
check("imshow aspect", lambda: plt.subplots()[1].imshow(np.zeros((2, 2, 3)) + 2))
check("hist empty", lambda: plt.subplots()[1].hist([]))
check("contour no levels", lambda: plt.subplots()[1].contour(np.ones((3, 3))))
check("pie negative", lambda: plt.subplots()[1].pie([1, -1]))
check("colorbar no mappable", lambda: fig.colorbar(plt.cm.ScalarMappable(), ax=ax))
