# Broken y axis with two subplots and diagonal break marks, for data with one huge outlier.
import numpy as np
import matplotlib.pyplot as plt

values = np.array([3, 5, 4, 6, 120, 5, 4, 7])
x = np.arange(values.size)
fig, (top, bottom) = plt.subplots(2, 1, sharex=True, figsize=(6, 4.5),
                                  gridspec_kw={"height_ratios": [1, 2], "hspace": 0.08})
top.bar(x, values, color="C0")
bottom.bar(x, values, color="C0")
top.set_ylim(100, 130)
bottom.set_ylim(0, 10)
top.spines.bottom.set_visible(False)
bottom.spines.top.set_visible(False)
top.xaxis.tick_top()
top.tick_params(labeltop=False)
bottom.xaxis.tick_bottom()
d = 0.5
kwargs = dict(marker=[(-1, -d), (1, d)], markersize=12, linestyle="none", color="k", mec="k", mew=1, clip_on=False)
top.plot([0, 1], [0, 0], transform=top.transAxes, **kwargs)
bottom.plot([0, 1], [1, 1], transform=bottom.transAxes, **kwargs)
fig.savefig("broken.png")
print(top.get_ylim(), bottom.get_ylim())
