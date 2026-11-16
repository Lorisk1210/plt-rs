# Error values containing NaN and zero, and yerr=None with fmt='none' to draw only bars.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(6)
y = np.array([1.0, 1.5, 1.2, 2.2, 2.0, 2.8])
err = np.array([0.2, np.nan, 0.0, 0.3, 0.25, 0.4])
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.errorbar(x, y, yerr=err, fmt="o", capsize=3)
a.set_title("NaN and zero errors")
b.errorbar(x, y, yerr=err * 0 + 0.3, fmt="none", ecolor="purple")
b.plot(x, y, "k.")
b.set_title("bars only")
print(len(a.containers[0].lines[2][0].get_segments()))
fig.savefig("nan_errors.png")
