# pandas.plotting.table helper drawing a DataFrame as a table on an axes.
import pandas as pd
import matplotlib.pyplot as plt
from pandas.plotting import table

df = pd.DataFrame({"plan": ["basic", "pro", "team"], "price": [5, 15, 40], "users": [1, 3, 10]}).set_index("plan")
fig, ax = plt.subplots(figsize=(5, 2))
ax.axis("off")
tbl = table(ax, df, loc="center", colWidths=[0.2, 0.2])
tbl.scale(1.2, 1.4)
fig.savefig("plans.png", bbox_inches="tight")
print(len(tbl.get_celld()))
fig.savefig("pandas_plotting_table_small.png", dpi=50)
print(fig.get_size_inches())
