# Series.plot of different kinds on one figure: line, bar, kde-free hist, and cumulative sum.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(12)
s = pd.Series(rng.normal(0, 1, 300).cumsum(), name="walk")
fig, axs = plt.subplots(2, 2, figsize=(9, 6))
s.plot(ax=axs[0, 0], title="line", color="k")
s.diff().plot.hist(ax=axs[0, 1], bins=25, title="hist of steps")
s.iloc[::30].plot.bar(ax=axs[1, 0], title="every 30th", rot=45)
s.rolling(20).std().plot(ax=axs[1, 1], title="rolling std", style="--")
fig.tight_layout()
print(round(float(s.iloc[-1]), 4))
fig.savefig("walk.png")
