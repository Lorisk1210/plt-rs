# Old code calling things matplotlib 3.11 deprecates or warns about; record each warning text.
import warnings
import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

warnings.simplefilter("always")
fig, ax = plt.subplots(figsize=(5, 3.5))
with warnings.catch_warnings(record=True) as caught:
    ax.boxplot([np.arange(10), np.arange(5)], vert=False)
    mpl.rcParams["axes.formatter.limits"] = [-5, 5]
    ax.plot(np.arange(5), np.arange(5) ** 2)
    ax.set_xticklabels(["a", "b", "c"])
    plt.legend(loc="best")
for w in caught:
    print(w.category.__name__, str(w.message)[:90])
fig.savefig("deprecated.png")
