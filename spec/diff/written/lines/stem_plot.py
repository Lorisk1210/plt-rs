# Discrete signal and its impulse response with stem(), customised markers and baseline.
import numpy as np
import matplotlib.pyplot as plt

n = np.arange(0, 20)
x = np.exp(-n / 5) * np.cos(n)
fig, (a, b) = plt.subplots(2, 1, figsize=(6, 5))
markerline, stemlines, baseline = a.stem(n, x)
plt.setp(markerline, color="red", markersize=5)
plt.setp(baseline, color="gray", linewidth=0.5)
a.set_title("signal")
b.stem(n, x ** 2, linefmt="C2:", markerfmt="D", basefmt="k-", bottom=0.1, orientation="vertical")
b.set_title("energy")
fig.tight_layout()
print(type(stemlines).__name__, markerline.get_marker())
fig.savefig("stem.png")
