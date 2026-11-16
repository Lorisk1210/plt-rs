# Numpy scalar types flowing into set_* calls and labels: int64 positions, float32 sizes, bool flags; printed back.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
vals = np.array([3, 7, 5], dtype=np.int64)
bars = ax.bar(np.arange(3, dtype=np.int32), vals, width=np.float32(0.6))
ax.set_xticks(np.arange(3), [f"n={v}" for v in vals])
ax.set_ylim(np.int64(0), np.float32(9.5))
ax.text(np.int64(1), np.float64(8), f"max={vals.max()}", fontsize=np.int32(12))
ax.grid(np.bool_(True))
line, = ax.plot([0, 2], [2, 2], lw=np.float16(2.5), visible=np.bool_(True))
print([type(b.get_height()).__name__ for b in bars], [type(b.get_width()).__name__ for b in bars][:1])
print(ax.get_ylim(), [type(v).__name__ for v in ax.get_ylim()], line.get_linewidth(), line.get_visible())
print([t.get_text() for t in ax.get_xticklabels()], ax.texts[0].get_fontsize())
fig.savefig("numpy_types.png", dpi=50)
