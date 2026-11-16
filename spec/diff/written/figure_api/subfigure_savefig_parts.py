# Save parts of a figure: draw subfigures and save the full figure and each subfigure area by bbox.
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 3))
left, right = fig.subfigures(1, 2)
left.subplots().plot([1, 2, 3])
left.suptitle("left part")
right.subplots().bar([1, 2], [2, 1])
right.suptitle("right part")
fig.savefig("whole.png")
fig.canvas.draw()
for name, sf in [("left", left), ("right", right)]:
    bb = sf.bbox.transformed(fig.dpi_scale_trans.inverted())
    fig.savefig(f"{name}.png", bbox_inches=bb)
    print(name, [round(v, 3) for v in bb.bounds])
