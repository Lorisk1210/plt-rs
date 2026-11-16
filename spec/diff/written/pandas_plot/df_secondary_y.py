# Pandas secondary_y for a second column on the right axis, with mark_right in the legend.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

df = pd.DataFrame({"revenue": [10, 12, 15, 14, 18, 21], "margin %": [5.0, 5.5, 6.1, 5.8, 6.9, 7.4]},
                  index=pd.period_range("2024Q1", periods=6, freq="Q"))
ax = df.plot(secondary_y="margin %", figsize=(7, 4), marker="o", mark_right=True)
ax.set_ylabel("revenue (M)")
ax.right_ax.set_ylabel("margin (%)")
print([t.get_text() for t in ax.get_legend().get_texts()])
plt.savefig("secondary.png")
current = plt.gcf()
print(len(current.axes), current.get_size_inches())
current.savefig("df_secondary_y_small.png", dpi=50)
