# The four built-in geographic projections side by side with a great-circle-like track.
import numpy as np
import matplotlib.pyplot as plt

lon = np.linspace(-np.pi, np.pi, 200)
track = 0.5 * np.sin(2 * lon)
fig = plt.figure(figsize=(10, 6))
for i, proj in enumerate(["aitoff", "hammer", "lambert", "mollweide"], start=1):
    ax = fig.add_subplot(2, 2, i, projection=proj)
    ax.plot(lon, track, color="navy")
    ax.set_title(proj, pad=12)
    ax.grid(True)
fig.tight_layout()
print([a.name for a in fig.axes])
fig.savefig("projections.png")
