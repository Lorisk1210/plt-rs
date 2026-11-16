# Log axis on data containing zeros and negatives: nonpositive='mask' vs 'clip'.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
y = np.array([0, 1, 10, -5, 100, 0, 1000, 50, 5, 0.5])
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.2))
a.plot(x, y, "o-")
a.set_yscale("log", nonpositive="mask")
a.set_title("mask")
b.plot(x, y, "o-")
b.set_yscale("log", nonpositive="clip")
b.set_title("clip")
fig.tight_layout()
print(a.get_ylim(), b.get_ylim())
fig.savefig("log_zeros.png")
