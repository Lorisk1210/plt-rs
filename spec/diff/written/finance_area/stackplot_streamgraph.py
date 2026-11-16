# Streamgraph with stackplot(baseline='wiggle') and a stacked area with 'sym' baseline.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(8)
x = np.arange(50)
layers = [np.abs(np.convolve(rng.normal(0, 1, 60), np.ones(11) / 11, mode="same"))[:50] * 10 + 1 for _ in range(5)]
fig, (a, b) = plt.subplots(2, 1, figsize=(8, 5), sharex=True)
a.stackplot(x, layers, baseline="wiggle", colors=plt.cm.Pastel1.colors)
a.set_title("wiggle")
b.stackplot(x, layers, baseline="sym", labels=[f"L{i}" for i in range(5)])
b.legend(ncols=5, fontsize=7, loc="upper left")
b.set_title("sym")
print(round(float(np.sum(layers)), 4))
fig.savefig("stream.png")
