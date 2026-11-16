# Irregularly sampled measurements: markers at sample times, gaps longer than a day broken.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

times = pd.to_datetime(["2024-05-01 08:00", "2024-05-01 14:00", "2024-05-02 09:30", "2024-05-05 10:00",
                        "2024-05-05 18:00", "2024-05-06 07:00", "2024-05-09 12:00"])
vals = np.array([3.2, 3.8, 3.5, 4.4, 4.1, 3.9, 4.8])
gap = np.r_[False, np.diff(times.values) > np.timedelta64(1, "D")]
y = vals.astype(float).copy()
fig, ax = plt.subplots(figsize=(7, 3.5))
seg_start = 0
for i in list(np.nonzero(gap)[0]) + [len(vals)]:
    ax.plot(times[seg_start:i], y[seg_start:i], "o-", color="C0")
    seg_start = i
fig.autofmt_xdate()
print(int(gap.sum()))
fig.savefig("irregular.png")
