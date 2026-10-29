"""Annotated heatmap: how strongly six daily weather measurements move
together, each cell labelled with its correlation."""

import numpy as np

import pltrs.pyplot as plt

names = ["temperature", "humidity", "wind", "pressure", "rain", "sunshine"]
correlation = np.array(
    [
        [1.00, -0.42, -0.18, 0.21, -0.25, 0.68],
        [-0.42, 1.00, 0.05, -0.35, 0.57, -0.71],
        [-0.18, 0.05, 1.00, -0.48, 0.31, -0.12],
        [0.21, -0.35, -0.48, 1.00, -0.62, 0.44],
        [-0.25, 0.57, 0.31, -0.62, 1.00, -0.53],
        [0.68, -0.71, -0.12, 0.44, -0.53, 1.00],
    ]
)

fig, ax = plt.subplots(figsize=(6.4, 5.2), layout="constrained")
image = ax.imshow(correlation, cmap="RdBu_r", vmin=-1, vmax=1)
fig.colorbar(image, ax=ax, label="correlation", shrink=0.8)
ax.set_xticks(range(len(names)), names, rotation=40, ha="right", rotation_mode="anchor")
ax.set_yticks(range(len(names)), names)
ax.grid(False)
ax.spines[:].set_visible(False)
for row in range(len(names)):
    for col in range(len(names)):
        value = correlation[row, col]
        ax.text(
            col,
            row,
            f"{value:.2f}",
            ha="center",
            va="center",
            fontsize=9,
            color="white" if abs(value) > 0.5 else "black",
        )
ax.set_title("Correlation between weather measurements")

fig.savefig("heatmap.png")
