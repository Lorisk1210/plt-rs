# A session collecting all warnings from typical calls (legend without labels, log limits, tight_layout) into a summary table.
import warnings
import numpy as np
import matplotlib.pyplot as plt

collected = []
with warnings.catch_warnings(record=True) as w:
    warnings.simplefilter("always")
    fig, axs = plt.subplots(1, 3, figsize=(10, 3))
    axs[0].plot([1, 2, 3]); axs[0].legend()
    axs[1].plot([1, 10, 100]); axs[1].set_yscale("log"); axs[1].set_ylim(0, 200)
    axs[2].plot([1, 2], "r", color="b")
    fig.tight_layout()
    collected = [(x.category.__name__, str(x.message)[:50]) for x in w]
t_fig, t_ax = plt.subplots(figsize=(8, 2))
t_ax.axis("off")
if collected:
    t_ax.table(cellText=collected, colLabels=["category", "message"], loc="center").set_fontsize(7)
print(collected)
fig.savefig("warned.png", dpi=40)
t_fig.savefig("warning_table.png", dpi=40)
