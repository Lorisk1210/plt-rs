# fontdict usage (as in old code) and set_* methods on returned Text objects.
import matplotlib.pyplot as plt

font = {"family": "serif", "color": "darkred", "weight": "normal", "size": 14}
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([0, 1, 2], [0, 1, 4])
ax.set_title("fontdict title", fontdict=font)
ax.set_xlabel("x", fontdict={"size": 12, "style": "italic"})
t = ax.text(0.2, 3, "editable text")
t.set_fontsize(16)
t.set_color("navy")
t.set_alpha(0.6)
t.set_position((0.1, 2.5))
t.set_text("edited text")
print(t.get_text(), t.get_position(), t.get_fontsize(), ax.title.get_family())
fig.savefig("fontdict.png")
