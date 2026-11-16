# Rows of subfigures, one per experiment, each with a row label and three panels sharing y.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 6), layout="constrained")
subfigs = fig.subfigures(3, 1, hspace=0.04)
rng = np.random.default_rng(2)
for row, subfig in enumerate(subfigs):
    subfig.suptitle(f"Experiment {row + 1}", x=0.02, ha="left", fontsize=10)
    axs = subfig.subplots(1, 3, sharey=True)
    for col, ax in enumerate(axs):
        ax.plot(rng.normal(0, 1, 40).cumsum(), color=f"C{row}")
        if row == 2:
            ax.set_xlabel(f"trial {col + 1}")
print([len(sf.axes) for sf in subfigs])
fig.savefig("experiments.png")
