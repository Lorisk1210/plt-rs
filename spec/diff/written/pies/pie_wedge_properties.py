# Restyle pie wedges after drawing: set_radius, edge width, and a highlight via set_hatch.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4.5, 4.5))
wedges, texts = ax.pie([10, 20, 30, 40], labels=list("ABCD"), startangle=90)
for w in wedges:
    w.set_edgecolor("white")
    w.set_linewidth(2)
wedges[2].set_radius(1.1)
wedges[2].set_hatch("xx")
texts[2].set_fontweight("bold")
ax.set_title("C highlighted")
print([(round(w.theta1, 2), round(w.theta2, 2), w.r) for w in wedges])
fig.savefig("highlight.png")
