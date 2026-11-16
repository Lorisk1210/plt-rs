# Readable labels on busy backgrounds with path effects (stroke outline, shadow).
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.patheffects as pe

img = np.random.default_rng(1).random((20, 30))
fig, ax = plt.subplots(figsize=(6, 4))
ax.imshow(img, cmap="gray")
ax.text(5, 5, "outlined", color="white", fontsize=18, path_effects=[pe.withStroke(linewidth=3, foreground="black")])
ax.text(5, 12, "shadow", color="yellow", fontsize=18, path_effects=[pe.withSimplePatchShadow(), pe.Normal()])
line, = ax.plot([0, 29], [18, 2], color="cyan", lw=2)
line.set_path_effects([pe.Stroke(linewidth=5, foreground="k"), pe.Normal()])
ax.axis("off")
print(len(ax.texts[0].get_path_effects()))
fig.savefig("path_effects.png")
