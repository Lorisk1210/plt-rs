# A simple energy-flow Sankey diagram with matplotlib.sankey (inputs and losses).
import matplotlib.pyplot as plt
from matplotlib.sankey import Sankey

fig, ax = plt.subplots(figsize=(7, 4))
sankey = Sankey(ax=ax, unit="%", scale=0.01, format="%.0f")
sankey.add(flows=[100, -60, -25, -15], labels=["fuel", "electricity", "heat loss", "other"],
           orientations=[0, 0, 1, -1], pathlengths=[0.5, 0.5, 0.3, 0.3], facecolor="lightsteelblue")
diagrams = sankey.finish()
ax.axis("off")
ax.set_title("Power plant")
print(len(diagrams), [t.get_text() for t in diagrams[0].texts])
fig.savefig("sankey.png")
