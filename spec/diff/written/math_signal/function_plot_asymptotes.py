# Plot tan(x) with asymptotes: mask large values so lines do not jump, dashed vertical asymptotes.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2 * np.pi, 2 * np.pi, 2000)
y = np.tan(x)
y[np.abs(y) > 10] = np.nan
fig, ax = plt.subplots(figsize=(7, 4))
ax.plot(x, y)
for k in range(-2, 2):
    ax.axvline(np.pi / 2 + k * np.pi, color="gray", ls="--", lw=0.8)
ax.set_ylim(-6, 6)
ax.set_xticks(np.arange(-2, 2.5, 0.5) * np.pi, [f"{v:g}π" if v else "0" for v in np.arange(-2, 2.5, 0.5)])
ax.axhline(0, color="k", lw=0.5)
print(int(np.isnan(y).sum()))
fig.savefig("tan.png")
