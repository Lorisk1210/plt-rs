# Time-lapse stack: montage of frames with timestamps, mean intensity over time, and a kymograph along one row.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:40j, 0:1:60j]
frames = [np.exp(-((x - 0.2 - 0.06 * t) ** 2 + (y - 0.5) ** 2) / 0.01) for t in range(10)]
fig = plt.figure(figsize=(12, 6))
gs = fig.add_gridspec(2, 5)
for t in range(5):
    ax = fig.add_subplot(gs[0, t])
    ax.imshow(frames[t * 2], cmap="viridis", vmin=0, vmax=1)
    ax.set_title(f"t={t * 2}s", fontsize=9)
    ax.axis("off")
ax_m = fig.add_subplot(gs[1, :2])
ax_m.plot([f.mean() for f in frames], "o-")
ax_m.set_xlabel("frame")
ax_k = fig.add_subplot(gs[1, 2:])
kymo = np.array([f[20] for f in frames])
ax_k.imshow(kymo, aspect="auto", cmap="viridis", origin="lower")
ax_k.set(xlabel="x [px]", ylabel="frame")
fig.tight_layout()
print([round(float(f.max()), 4) for f in frames[:3]], kymo.shape)
fig.savefig("montage.png", dpi=50)
