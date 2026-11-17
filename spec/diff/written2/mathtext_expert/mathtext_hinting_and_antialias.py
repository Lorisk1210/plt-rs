# Mathtext through the Agg parser with text.antialiased on/off and text.hinting variants; ink counts in the rendered images.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

expr = r"$\frac{d}{dx}\left[\int_0^x g(t)\,dt\right] = g(x)$"
results = {}
for aa in (True, False):
    for hinting in ("none", "auto", "native", "either", "default"):
        with plt.rc_context({"text.antialiased": aa, "text.hinting": hinting, "text.hinting_factor": 8}):
            fig = plt.figure(figsize=(4, 1), dpi=100)
            t = fig.text(0.02, 0.3, expr, fontsize=14)
            fig.canvas.draw()
            img = np.asarray(fig.canvas.buffer_rgba())[..., 0]
            ink = int((img < 250).sum())
            grey = int(((img > 5) & (img < 250)).sum())
            results[(aa, hinting)] = (ink, grey, round(t.get_window_extent().width, 2))
            plt.close(fig)
for key, val in results.items():
    print(key, val)
print(results[(False, "none")][1] == 0, results[(True, "none")][1] > 0)
fig = plt.figure(figsize=(4, 1), dpi=100)
t = fig.text(0.02, 0.3, expr, fontsize=14, antialiased=False)
fig.canvas.draw()
print(t.get_antialiased(), int((np.asarray(fig.canvas.buffer_rgba())[..., 0] < 250).sum()))
t.set_antialiased(True)
fig.canvas.draw()
print(t.get_antialiased(), int((np.asarray(fig.canvas.buffer_rgba())[..., 0] < 250).sum()))
fig.savefig("hinting.png")
