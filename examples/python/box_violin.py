"""Box plots and violin plots: the exam scores of four classes taught four
ways, summarised side by side on a shared axis."""

import numpy as np

import pltrs.pyplot as plt
from samples import Samples

rng = Samples(11)
methods = ["lecture", "seminar", "project", "online"]
spreads = [(68, 11), (74, 8), (77, 13), (63, 15)]
scores = [np.clip(rng.normal(120, mean, sd), 0, 100) for mean, sd in spreads]
colors = ["C0", "C1", "C2", "C3"]

fig, (left, right) = plt.subplots(1, 2, figsize=(7.2, 4.5), sharey=True, layout="constrained")
boxes = left.boxplot(
    scores,
    tick_labels=methods,
    patch_artist=True,
    widths=0.55,
    medianprops=dict(color="white", linewidth=1.5),
)
for box, color in zip(boxes["boxes"], colors):
    box.set_facecolor(color)
left.set_title("Box plot")
left.set_ylabel("score")

right.violinplot(scores, showmedians=True, facecolor=colors, linecolor="0.35")
right.set_xticks([1, 2, 3, 4], methods)
right.set_title("Violin plot")
fig.suptitle("Exam scores by teaching method")

fig.savefig("box_violin.png")
