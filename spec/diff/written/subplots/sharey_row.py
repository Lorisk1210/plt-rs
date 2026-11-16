# Compare score distributions of three classes side by side with a shared y axis ("row" sharing).
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
classes = {"A": rng.normal(70, 8, 120), "B": rng.normal(64, 12, 120), "C": rng.normal(75, 5, 120)}
fig, axes = plt.subplots(2, 3, sharey="row", sharex="col", figsize=(9, 5))
for col, (name, scores) in enumerate(classes.items()):
    axes[0, col].hist(scores, bins=12, color=f"C{col}")
    axes[0, col].set_title(f"class {name}")
    axes[1, col].boxplot(scores, vert=False)
    axes[1, col].set_xlabel("score")
axes[0, 0].set_ylabel("students")
for ax in axes.flat:
    ax.label_outer()
print([a.get_ylim() for a in axes[0]])
fig.savefig("classes.png")
