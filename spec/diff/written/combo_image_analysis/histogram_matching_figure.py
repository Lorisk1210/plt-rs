# Contrast adjustments: original, gamma-corrected and clipped versions with their histograms and the transfer curves.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(82)
img = np.clip(rng.beta(2, 5, (80, 120)) + np.linspace(0, 0.2, 120), 0, 1)
versions = {"original": img, "gamma 0.5": img ** 0.5, "stretched": np.clip((img - 0.1) / 0.5, 0, 1)}
fig, axs = plt.subplots(3, 3, figsize=(10, 9))
for col, (name, v) in enumerate(versions.items()):
    axs[0, col].imshow(v, cmap="gray", vmin=0, vmax=1)
    axs[0, col].set_title(name)
    axs[0, col].axis("off")
    axs[1, col].hist(v.ravel(), bins=50, color="0.4")
    axs[1, col].set_xlim(0, 1)
t = np.linspace(0, 1, 100)
for ax, f in zip(axs[2], [t, t ** 0.5, np.clip((t - 0.1) / 0.5, 0, 1)]):
    ax.plot(t, f)
    ax.set_aspect("equal")
fig.tight_layout()
print({k: round(float(v.mean()), 4) for k, v in versions.items()})
fig.savefig("contrast.png", dpi=50)
