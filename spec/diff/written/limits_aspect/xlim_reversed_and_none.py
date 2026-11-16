# Common limit calls: set_xlim with a tuple, reversed limits, xlim() getter, ylim with keywords.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(400, 4000, 200)
absorb = np.exp(-((x - 1700) / 60) ** 2) + 0.6 * np.exp(-((x - 2900) / 120) ** 2)
plt.figure(figsize=(7, 3.5))
plt.plot(x, absorb, "k")
plt.xlim((4000, 400))
plt.ylim(ymin=-0.05)
print(plt.xlim(), plt.ylim())
plt.xlabel("wavenumber [cm$^{-1}$]")
plt.ylabel("absorbance")
plt.title("IR spectrum")
plt.savefig("ir.png")
print(plt.gca().get_autoscale_on())
