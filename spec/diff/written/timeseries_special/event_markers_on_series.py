# Vertical event lines with rotated labels at the top of a time series (releases, incidents).
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

idx = pd.date_range("2024-01-01", periods=120, freq="D")
users = pd.Series(1000 + np.arange(120) * 5 + 50 * np.sin(np.arange(120) / 5), index=idx)
events = {"v2 launch": "2024-02-01", "outage": "2024-03-03", "campaign": "2024-04-10"}
fig, ax = plt.subplots(figsize=(8, 4))
ax.plot(users.index, users)
for name, day in events.items():
    t = pd.Timestamp(day)
    ax.axvline(t, color="gray", ls="--", lw=0.8)
    ax.text(t, 1.01, name, rotation=45, transform=ax.get_xaxis_transform(), fontsize=8, va="bottom")
fig.subplots_adjust(top=0.8)
print(len(ax.texts))
fig.savefig("events.png")
