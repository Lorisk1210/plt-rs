# Label each point with its name, offset slightly, for a small set of countries.
import matplotlib.pyplot as plt

countries = {"NO": (89, 82.9), "US": (76, 77.5), "IN": (7, 70.8), "BR": (15, 75.3), "JP": (34, 84.5),
             "NG": (5, 54.7), "DE": (54, 80.6), "CN": (23, 78.0)}
fig, ax = plt.subplots(figsize=(6, 4))
for code, (gdp, life) in countries.items():
    ax.scatter(gdp, life, color="teal")
    ax.annotate(code, (gdp, life), textcoords="offset points", xytext=(4, 4), fontsize=9)
ax.set_xscale("log")
ax.set_xlabel("GDP per capita (k$, log)")
ax.set_ylabel("life expectancy")
ax.grid(True, which="both", alpha=0.3)
print(len(ax.collections), len(ax.texts))
fig.savefig("countries.png")
