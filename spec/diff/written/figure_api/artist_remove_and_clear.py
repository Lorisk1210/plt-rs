# Remove individual artists (line, text, patch, collection, legend) and redraw after each removal.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
line, = ax.plot([0, 1], [0, 1], label="line")
txt = ax.text(0.5, 0.8, "text")
patch = ax.axvspan(0.2, 0.4, alpha=0.3)
coll = ax.scatter([0.5], [0.5], s=100)
leg = ax.legend()
for art in [line, txt, patch, coll, leg]:
    art.remove()
    fig.canvas.draw()
    print(type(art).__name__, len(ax.lines), len(ax.texts), len(ax.patches), len(ax.collections), ax.get_legend())
fig.savefig("emptied.png")
