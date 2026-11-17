# set_aspect with adjustable box/datalim, anchors and box_aspect on shared axes; final limits and positions for each mode.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for adjustable in ("box", "datalim"):
    for anchor in ("C", "SW", "NE"):
        fig, ax = plt.subplots(figsize=(5, 3), dpi=60)
        ax.plot([0, 4], [0, 1])
        ax.set_aspect(1, adjustable=adjustable, anchor=anchor)
        fig.canvas.draw()
        print(adjustable, anchor, [round(v, 3) for v in ax.get_position().bounds], [round(v, 3) for v in ax.get_xlim()],
              [round(v, 3) for v in ax.get_ylim()])
        plt.close(fig)
fig, (a1, a2) = plt.subplots(1, 2, sharey=True, figsize=(6, 3))
a1.set_box_aspect(1)
a2.set_box_aspect(0.5)
fig.canvas.draw()
print([round(v, 3) for v in a1.get_position().bounds], [round(v, 3) for v in a2.get_position().bounds])
try:
    a1.set_aspect(2, adjustable="datalim")
    a1.set_xlim(0, 1)
    a2.set_xlim(0, 1)
    fig.canvas.draw()
    print("datalim on shared", a1.get_adjustable())
except RuntimeError as e:
    print("RuntimeError", str(e)[:60])
print(a1.get_box_aspect(), a1.get_aspect(), a1.get_anchor())
fig.set_size_inches(8, 3)
fig.canvas.draw()
print([round(v, 3) for v in a1.get_position().bounds], [round(v, 3) for v in a1.get_position(original=True).bounds])
