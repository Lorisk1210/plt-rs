# Pandas plot with style strings per column, logy, yerr from another frame and xticks rotation.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

x = np.arange(1, 9)
df = pd.DataFrame({"fast": 2.0 ** x, "slow": 1.5 ** x, "linear": 10 * x}, index=x)
err = df * 0.1
ax = df.plot(style=["o-", "s--", "^:"], logy=True, yerr=err, capsize=3, figsize=(6, 4), rot=30)
ax.set_xlabel("n")
print(ax.get_yscale(), len(ax.containers))
plt.savefig("styles.png")
current = plt.gcf()
print(len(current.axes), current.get_size_inches())
current.savefig("df_plot_styles_errorbars_small.png", dpi=50)
