# Inspect a style from the library dict and apply part of it with rcParams.update.
import matplotlib.pyplot as plt
import matplotlib.style as mstyle

ggplot = mstyle.library["ggplot"]
print(sorted(ggplot)[:5])
print(ggplot["axes.facecolor"], ggplot["patch.edgecolor"])
plt.rcParams.update({k: v for k, v in ggplot.items() if k.startswith("axes.")})
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.hist([1, 2, 2, 3, 3, 3, 4, 4, 5], bins=5)
ax.set_title("ggplot axes only")
fig.savefig("partial_ggplot.png")
print(plt.rcParams["axes.facecolor"])
