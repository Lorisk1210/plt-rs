# Axes titles left/center/right with pad, y, rcParams axes.titley/titlepad, and titles moving above top ticks and x labels.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(6, 5), dpi=72)
a, b, c, d = axs.flat
a.set_title("left", loc="left")
a.set_title("center")
a.set_title("right", loc="right", pad=20)
b.set_title("y=1.2", y=1.2)
c.xaxis.tick_top()
c.set_xlabel("top label")
c.xaxis.set_label_position("top")
c.set_title("above ticks")
with plt.rc_context({"axes.titley": 1.0, "axes.titlepad": 2, "axes.titlelocation": "right", "axes.titleweight": "bold"}):
    d.set_title("rc title")
fig.canvas.draw()
r = fig.canvas.get_renderer()
for ax in (a, b, c, d):
    for t in (ax.title, ax._left_title, ax._right_title):
        if t.get_text():
            print(repr(t.get_text()), [round(v, 1) for v in t.get_window_extent(r).bounds], round(t.get_position()[1], 3))
print(d.title.get_text(), d._right_title.get_text(), d._right_title.get_weight())
print(a.get_title(loc="right"), a.get_title(), a.get_title("left"))
fig.suptitle("suptitle", x=0.1, ha="left", y=0.99)
fig.supxlabel("supx")
fig.supylabel("supy")
fig.canvas.draw()
print([round(v, 1) for v in fig._suptitle.get_window_extent(r).bounds], [round(v, 1) for v in fig._supxlabel.get_window_extent(r).bounds])
fig.savefig("titles.png")
