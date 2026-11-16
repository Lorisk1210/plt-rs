# Per-point RGBA colours computed by hand (alpha by density) and edge colours by a rule.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
pts = rng.normal(0, 1, (300, 2))
dist = np.hypot(*pts.T)
rgba = np.zeros((300, 4))
rgba[:, 0] = 0.1
rgba[:, 1] = 0.4
rgba[:, 2] = 0.8
rgba[:, 3] = np.clip(1 - dist / 3, 0.1, 1)
edge = np.where(dist > 2, "red", "none")
fig, ax = plt.subplots(figsize=(5, 5))
ax.scatter(pts[:, 0], pts[:, 1], c=rgba, edgecolors=edge, s=25)
ax.set_aspect("equal")
fc = ax.collections[0].get_facecolors()
print(fc.shape, np.round(fc[0], 3))
fig.savefig("rgba.png")
