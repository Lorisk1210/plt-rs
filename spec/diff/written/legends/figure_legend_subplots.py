# One figure-level legend shared by four subplots, collected from the first axes.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(7, 5), sharex=True)
x = np.linspace(0, 2, 60)
for ax, scale in zip(axs.flat, [1, 2, 3, 4]):
    ax.plot(x, scale * x, label="linear")
    ax.plot(x, scale * x ** 2, label="quadratic")
    ax.plot(x, scale * np.sqrt(x), label="root")
    ax.set_title(f"scale {scale}")
handles, labels = axs[0, 0].get_legend_handles_labels()
fig.legend(handles, labels, loc="lower center", ncol=3)
fig.tight_layout(rect=(0, 0.07, 1, 1))
print(len(fig.legends), labels)
fig.savefig("figure_legend.png")
