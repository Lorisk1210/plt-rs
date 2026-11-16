# Population growth on a log y axis compared with linear, with semilogy and loglog helpers.
import numpy as np
import matplotlib.pyplot as plt

years = np.arange(1800, 2030, 10)
pop = 1.0 * np.exp(0.0085 * (years - 1800)) * (1 + 0.5 * (years > 1950))
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
axs[0].plot(years, pop)
axs[0].set_title("linear")
axs[1].semilogy(years, pop)
axs[1].set_title("semilogy")
n = np.logspace(1, 6, 30)
axs[2].loglog(n, n * np.log2(n), label="n log n", base=2)
axs[2].loglog(n, n ** 2, label="n²", base=2)
axs[2].legend()
fig.tight_layout()
print(axs[1].get_yscale(), axs[2].get_xscale(), axs[2].xaxis.get_transform().base)
fig.savefig("log.png")
