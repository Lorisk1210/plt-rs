# Place axes by hand with fig.add_axes in figure fractions: main plot plus a small overview.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 100, 1000)
y = np.sin(x / 3) * np.exp(-x / 60)
fig = plt.figure(figsize=(7, 4))
main = fig.add_axes([0.1, 0.12, 0.85, 0.78])
main.plot(x[:200], y[:200])
main.set_title("Zoomed start of signal")
overview = fig.add_axes([0.62, 0.62, 0.3, 0.22])
overview.plot(x, y, lw=0.6, color="gray")
overview.axvspan(0, 20, color="orange", alpha=0.3)
overview.set_xticks([])
overview.set_yticks([])
print(main.get_position().bounds, overview.get_position().bounds)
fig.savefig("overview.png")
