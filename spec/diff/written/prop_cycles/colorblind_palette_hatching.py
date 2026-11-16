# Colour-blind-friendly style ('tableau-colorblind10') with markers and hatch distinctions.
import numpy as np
import matplotlib.pyplot as plt

plt.style.use("tableau-colorblind10")
x = np.arange(5)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
for k, m in enumerate("osD^v"):
    a.plot(x, x * (k + 1) / 2, marker=m, label=f"group {k}")
a.legend()
for k, h in enumerate(["", "//", "..", "xx"]):
    b.bar(x + k * 0.2, x + k, 0.2, hatch=h, edgecolor="k", label=f"set {k}")
b.legend()
print([l.get_color() for l in a.lines][:3])
fig.savefig("colorblind.png")
