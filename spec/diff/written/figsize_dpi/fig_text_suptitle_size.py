# Figure-level texts (suptitle, supxlabel, footnote) on a wide figure, sizes in points and keywords.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(10, 3), sharey=True)
for k, ax in enumerate(axs):
    ax.plot(np.arange(10), np.arange(10) * (k + 1))
    ax.set_title(f"rate {k + 1}")
fig.suptitle("Three rates", fontsize=16, fontweight="bold")
fig.supxlabel("time step")
fig.supylabel("value")
fig.text(0.99, 0.01, "source: simulation", ha="right", va="bottom", fontsize=7, color="gray")
fig.subplots_adjust(bottom=0.2, top=0.8)
print(fig._suptitle.get_text(), fig._supxlabel.get_fontsize())
fig.savefig("rates.png")
