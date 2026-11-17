# Numpy RuntimeWarnings surfacing through plotting (log of zero, 0/0 in norms, empty means) and errstate silencing them.
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import colors


def record(label, fn):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            fn()
        except Exception as e:
            print(label, "raised", type(e).__name__)
    print(label, sorted({(w.category.__name__, str(w.message)[:40]) for w in caught}))


fig, ax = plt.subplots()
record("log of zeros", lambda: (ax.set_yscale("log"), ax.plot([0, 1, 10], [0, 1, 10]), fig.canvas.draw()))
record("norm constant", lambda: colors.Normalize()(np.array([1.0, 1.0, 1.0])))
record("lognorm zero", lambda: colors.LogNorm()(np.array([0.0, 1.0, 10.0])))
record("imshow nan", lambda: (plt.subplots()[1].imshow(np.full((3, 3), np.nan)), plt.gcf().canvas.draw()))
record("hist nan", lambda: plt.subplots()[1].hist([1.0, np.nan, 2.0]))
record("scatter inf", lambda: (plt.subplots()[1].scatter([1, np.inf], [1, 2]), plt.gcf().canvas.draw()))
record("contour const", lambda: plt.subplots()[1].contour(np.zeros((4, 4))))
with np.errstate(all="ignore"):
    record("errstate log", lambda: (ax.plot([0, 0.5], [0, 2]), fig.canvas.draw()))
record("boxplot empty", lambda: plt.subplots()[1].boxplot([[], [1, 2, 3]]))
record("violin const", lambda: plt.subplots()[1].violinplot([[1.0, 1.0, 1.0]]))
print(np.geterr())
plt.close("all")
