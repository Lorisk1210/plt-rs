# A save_all(fig, stem) function writing png/pdf/svg with consistent options and returning what it wrote.
import os
import numpy as np
import matplotlib.pyplot as plt


def save_all(fig, stem, formats=("png", "pdf", "svg"), **kwargs):
    written = []
    options = dict(dpi=60, bbox_inches="tight", metadata=None)
    options.update(kwargs)
    for fmt in formats:
        meta = {"png": {"Software": None}, "pdf": {"CreationDate": None}, "svg": {"Date": None}}[fmt]
        fig.savefig(f"{stem}.{fmt}", **{**options, "metadata": meta})
        written.append(f"{stem}.{fmt}")
    return written


fig, ax = plt.subplots(figsize=(5, 3))
ax.plot(np.arange(10), np.arange(10) ** 2, "o-")
ax.legend(["squares"], loc="upper left", bbox_to_anchor=(1, 1))
print(save_all(fig, "figure"))
print(save_all(fig, "figure_transparent", formats=("png",), transparent=True))
print(sorted(f for f in os.listdir(".") if f.startswith("figure")))
