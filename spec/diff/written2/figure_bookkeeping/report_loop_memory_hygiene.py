# A loop producing 40 report pages the way scripts do (subplots, savefig, close), counting open figures and warnings as it goes.
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

open_counts = []
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    for k in range(40):
        fig, ax = plt.subplots(figsize=(2, 1.5), dpi=40)
        ax.plot(np.arange(10), np.arange(10) * k)
        ax.set_title(f"page {k}")
        fig.savefig(f"page_{k:02d}.png")
        if k % 2 == 0:
            plt.close(fig)
        open_counts.append(len(plt.get_fignums()))
print(open_counts[::5], open_counts[-1])
print(sorted({str(w.message)[:70] for w in caught}))
print(plt.rcParams["figure.max_open_warning"])
plt.close("all")
with plt.rc_context({"figure.max_open_warning": 5}), warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    figs = [plt.figure() for _ in range(8)]
print(len(caught), [str(w.message)[:40] for w in caught][:1])
for f in figs:
    plt.close(f)
with plt.rc_context({"figure.max_open_warning": 0}), warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    figs = [plt.figure() for _ in range(30)]
print(len(caught), len(plt.get_fignums()))
plt.close("all")
import os
print(len([f for f in os.listdir(".") if f.startswith("page_")]))
