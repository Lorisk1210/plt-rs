# Horizontal bar chart of the top 10 items sorted descending, with the value at the bar end.
import matplotlib.pyplot as plt

data = {"Paris": 2.1, "Berlin": 3.6, "Madrid": 3.3, "Rome": 2.8, "Vienna": 1.9, "Warsaw": 1.8,
        "Budapest": 1.7, "Hamburg": 1.9, "Bucharest": 1.8, "Barcelona": 1.6, "Munich": 1.5}
top = sorted(data.items(), key=lambda kv: kv[1], reverse=True)[:10]
names, pops = zip(*top)
fig, ax = plt.subplots(figsize=(6, 4.5))
bars = ax.barh(names, pops, color="teal")
ax.invert_yaxis()
ax.bar_label(bars, fmt="%.1f M", padding=3)
ax.set_xlim(0, 4.2)
ax.set_xlabel("population (millions)")
ax.spines[["right", "top"]].set_visible(False)
fig.tight_layout()
print(names)
fig.savefig("cities.png")
