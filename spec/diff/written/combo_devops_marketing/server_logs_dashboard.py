# Server log dashboard: requests per minute with error spikes, status-code stacked area, latency histogram (log x), top endpoints.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(200)
n = 20000
logs = pd.DataFrame({"ts": pd.Timestamp("2024-07-01 12:00") + pd.to_timedelta(np.sort(rng.uniform(0, 7200, n)), unit="s"),
                     "status": rng.choice([200, 301, 404, 500], n, p=[0.85, 0.05, 0.07, 0.03]),
                     "latency": rng.lognormal(4, 0.6, n),
                     "endpoint": rng.choice(["/api/a", "/api/b", "/home", "/login", "/search"], n, p=[0.3, 0.25, 0.2, 0.1, 0.15])})
per_min = logs.set_index("ts").resample("1min")
counts = per_min["status"].value_counts().unstack(fill_value=0)
fig = plt.figure(figsize=(12, 8))
a = fig.add_subplot(2, 1, 1)
a.stackplot(counts.index, *[counts[c] for c in counts.columns], labels=[str(c) for c in counts.columns])
a.legend(loc="upper left", ncols=4)
b = fig.add_subplot(2, 2, 3)
b.hist(logs["latency"], bins=np.logspace(1, 4, 40))
b.set_xscale("log")
b.axvline(np.percentile(logs["latency"], 99), color="r", label="p99")
b.legend()
c = fig.add_subplot(2, 2, 4)
top = logs["endpoint"].value_counts()
c.barh(top.index, top.values)
c.invert_yaxis()
fig.tight_layout()
print(counts.sum().to_dict(), round(float(np.percentile(logs["latency"], 99)), 2))
fig.savefig("logs.png", dpi=50)
