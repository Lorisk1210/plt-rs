# DataFrame.boxplot(by=...) for a column per group, and the automatic suptitle it adds.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(16)
df = pd.DataFrame({"line": rng.choice(["L1", "L2", "L3"], 240), "defects": rng.poisson(3, 240)})
df.loc[df.line == "L3", "defects"] += 2
ax = df.boxplot(column="defects", by="line", figsize=(6, 4), grid=False)
fig = ax.get_figure()
print(fig._suptitle.get_text() if fig._suptitle else None)
ax.set_title("defects per production line")
fig.suptitle("")
fig.savefig("defects.png")
print(df.groupby("line")["defects"].median().to_dict())
