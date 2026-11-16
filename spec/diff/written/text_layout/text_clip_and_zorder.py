# Text that falls outside the axes (clip_on False by default) vs clip_on=True, and zorder over lines.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([0, 1], [0, 1], lw=8, color="orange", zorder=2)
ax.text(0.5, 0.5, "behind", fontsize=20, zorder=1, ha="center")
ax.text(0.5, 0.3, "in front", fontsize=20, zorder=3, ha="center")
ax.text(1.05, 0.5, "outside, shown", clip_on=False)
ax.text(1.05, 0.2, "outside, clipped", clip_on=True)
ax.set_xlim(0, 1)
fig.subplots_adjust(right=0.7)
print([t.get_clip_on() for t in ax.texts])
fig.savefig("clipping.png")
