# pandas.plotting lag_plot and autocorrelation_plot of a seasonal series.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from pandas.plotting import lag_plot, autocorrelation_plot

rng = np.random.default_rng(1)
s = pd.Series(np.sin(np.arange(200) * 2 * np.pi / 12) + rng.normal(0, 0.3, 200))
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
lag_plot(s, lag=1, ax=a, s=8)
autocorrelation_plot(s, ax=b)
print(round(float(s.autocorr(12)), 4))
fig.savefig("lags.png")
