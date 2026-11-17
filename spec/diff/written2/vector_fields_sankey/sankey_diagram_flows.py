# Sankey diagrams: flows, orientations, labels, connecting two diagrams with prior/connect, and the returned diagram data.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.sankey import Sankey

fig = plt.figure(figsize=(6, 5), dpi=60)
ax = fig.add_subplot(xticks=[], yticks=[])
sk = Sankey(ax=ax, scale=0.01, offset=0.2, head_angle=120, format="%.0f", unit=" kW")
sk.add(flows=[100, -40, -35, -25], labels=["in", "heat", "work", ""], orientations=[0, 1, 0, -1],
       pathlengths=[0.3, 0.25, 0.25, 0.25], label="first")
sk.add(flows=[35, -20, -15], labels=["", "a", "b"], orientations=[0, 0, -1], prior=0, connect=(2, 0), label="second")
diagrams = sk.finish()
fig.canvas.draw()
for d in diagrams:
    print(type(d.patch).__name__, d.flows.tolist(), d.angles, [t.get_text() for t in d.texts], len(d.patch.get_path().vertices))
    print([[round(v, 3) for v in tip] for tip in d.tips[:2]])
print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
try:
    Sankey(ax=ax).add(flows=[1, -1], orientations=[0])
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.savefig("sankey.png")
fig2 = plt.figure(figsize=(4, 3), dpi=60)
ax2 = fig2.add_subplot()
d2 = Sankey(ax=ax2, flows=[0.25, 0.15, 0.6, -0.2, -0.15, -0.05, -0.5, -0.1], labels=list("abcdefgh"),
            orientations=[-1, 1, 0, 1, 1, 1, 0, -1], rotation=90, gap=0.3, radius=0.1).finish()
fig2.canvas.draw()
print(len(d2), d2[0].angles, len(d2[0].patch.get_path().vertices), [t.get_text() for t in d2[0].texts][:3])
print([round(v, 3) for v in ax2.get_xlim()])
fig2.savefig("sankey2.png")
