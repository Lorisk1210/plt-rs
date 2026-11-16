# groupby on two keys, unstack, and plot the resulting wide table as grouped bars via pandas.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(9)
df = pd.DataFrame({"region": rng.choice(["EU", "US", "APAC"], 400), "channel": rng.choice(["online", "retail"], 400),
                   "amount": rng.gamma(2, 50, 400)})
wide = df.groupby(["region", "channel"])["amount"].sum().unstack()
ax = wide.plot(kind="bar", figsize=(6, 4), rot=0, color=["#4e79a7", "#f28e2b"])
ax.set_ylabel("total amount")
ax.legend(title="channel")
print(wide.round(1).to_string())
plt.tight_layout()
plt.savefig("regions.png")
