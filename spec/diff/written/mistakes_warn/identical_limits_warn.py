# Plotting a single point or setting identical limits warns about a singular transform; see the expansion.
import warnings
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, axs = plt.subplots(1, 3, figsize=(9, 3))
axs[0].plot([5], [5], "o")
with warnings.catch_warnings(record=True) as w:
    axs[1].set_xlim(3, 3)
    axs[2].set_ylim(0.0, 0.0)
for item in w:
    print(item.category.__name__, str(item.message))
for ax in axs:
    print([round(v, 4) for v in ax.get_xlim()], [round(v, 4) for v in ax.get_ylim()])
fig.savefig("singular.png")
