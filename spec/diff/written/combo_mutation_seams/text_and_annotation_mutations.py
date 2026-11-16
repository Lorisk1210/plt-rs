# Move and restyle annotations after creation (xy, xyann, arrow props, text) and read their positions back.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
ax.set_xlim(0, 10); ax.set_ylim(0, 10)
ann = ax.annotate("start", xy=(2, 2), xytext=(4, 6), arrowprops=dict(arrowstyle="->"))
ann.xy = (5, 5)
ann.set_position((8, 8))
ann.xyann = (7, 2)
ann.set_text("moved")
ann.arrow_patch.set_color("red")
ann.set_fontsize(14)
fig.canvas.draw()
print(ann.xy, ann.get_position(), ann.xyann, ann.get_text(), ann.arrow_patch.get_edgecolor(), ann.get_fontsize())
t = ax.text(1, 1, "label")
t.set_x(3); t.set_y(4); t.set_ha("right"); t.set_va("top")
print(t.get_position(), t.get_ha(), t.get_va())
fig.savefig("annotation_mutations.png", dpi=50)
