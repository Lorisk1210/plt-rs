# Constrained layout with a legend placed outside right via loc='outside right upper'.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 1, figsize=(7, 5), layout="constrained", sharex=True)
x = np.arange(2010, 2025)
for name, growth in [("solar", 0.25), ("wind", 0.12), ("hydro", 0.01), ("coal", -0.05)]:
    axs[0].plot(x, 100 * (1 + growth) ** (x - 2010), label=name)
    axs[1].plot(x, growth * 100 + 0 * x, label=name)
axs[0].set_yscale("log")
axs[0].set_ylabel("index")
axs[1].set_ylabel("growth %")
fig.legend(*axs[0].get_legend_handles_labels(), loc="outside right upper", title="source")
fig.suptitle("Energy mix")
fig.savefig("energy.png")
print(len(fig.legends))
