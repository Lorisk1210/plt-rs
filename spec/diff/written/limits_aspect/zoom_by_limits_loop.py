# Save three zoom levels of the same plot by changing limits in a loop.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-10, 10, 2000)
y = np.sinc(x) + 0.02 * np.sin(40 * x)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(x, y, lw=0.8)
for i, half in enumerate([10, 3, 0.5]):
    ax.set_xlim(-half, half)
    ax.relim()
    ax.autoscale_view(scalex=False)
    ax.set_title(f"zoom ±{half}")
    fig.savefig(f"zoom_{i}.png")
    print(half, [round(v, 4) for v in ax.get_ylim()])
