# Pickle the same figure with every protocol, the sizes relative to each other, and loading after pyplot closed everything.
import pickle
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(4, 4))
for k, ax in enumerate(axs.flat):
    ax.plot(np.linspace(0, 1, 200), np.linspace(0, 1, 200) ** (k + 1))
    ax.set_title(f"p{k}")
blobs = {p: pickle.dumps(fig, protocol=p) for p in range(2, pickle.HIGHEST_PROTOCOL + 1)}
sizes = {p: len(b) for p, b in blobs.items()}
print(sorted(blobs), min(sizes.values()) > 5000)
print(sizes[pickle.HIGHEST_PROTOCOL] <= sizes[2])
plt.close("all")
print(plt.get_fignums())
restored = [pickle.loads(b) for b in blobs.values()]
print(plt.get_fignums()[:2], len(plt.get_fignums()) == len(restored))
print([len(f.axes) for f in restored], [f.axes[3].get_title() for f in restored][:2])
for f in restored:
    f.canvas.draw()
print(all(f.axes[0].lines[0].get_xydata().shape == (200, 2) for f in restored))
fig2 = pickle.loads(blobs[2])
fig2.axes[0].set_title("changed")
print(restored[0].axes[0].get_title(), fig2.axes[0].get_title())
plt.close("all")
print(plt.get_fignums())
try:
    pickle.loads(blobs[2][:100])
except Exception as e:
    print(type(e).__name__)
