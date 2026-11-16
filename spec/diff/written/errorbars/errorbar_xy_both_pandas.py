# Both x and y errors from DataFrame columns, coloured by group via separate errorbar calls.
import pandas as pd
import matplotlib.pyplot as plt

df = pd.DataFrame({
    "temp": [10, 20, 30, 40, 10, 20, 30, 40],
    "rate": [1.1, 1.9, 3.2, 4.1, 0.8, 1.5, 2.1, 2.9],
    "dt": [0.5, 0.5, 0.8, 0.8, 0.5, 0.5, 0.8, 0.8],
    "dr": [0.1, 0.15, 0.2, 0.3, 0.1, 0.1, 0.15, 0.2],
    "catalyst": ["Pt", "Pt", "Pt", "Pt", "Ni", "Ni", "Ni", "Ni"],
})
fig, ax = plt.subplots(figsize=(5.5, 4))
for name, g in df.groupby("catalyst"):
    ax.errorbar(g["temp"], g["rate"], xerr=g["dt"], yerr=g["dr"], fmt="o-", capsize=2, label=name)
ax.legend(title="catalyst")
ax.set_xlabel("temperature [°C]")
ax.set_ylabel("rate")
print(df.groupby("catalyst")["rate"].mean().round(3).to_dict())
fig.savefig("catalysts.png")
