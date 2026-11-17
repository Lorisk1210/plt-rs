# Text wrapping: wrap=True in axes and figures, widths at several figure sizes, rotation with wrap, and _get_wrapped_text results.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

msg = "The quick brown fox jumps over the lazy dog while the lazy dog keeps sleeping in the afternoon sun."
fig = plt.figure(figsize=(4, 3), dpi=72)
t1 = fig.text(0.05, 0.9, msg, wrap=True, va="top")
t2 = fig.text(0.5, 0.5, msg, wrap=True, ha="center", rotation=20)
t3 = fig.text(0.95, 0.1, msg, wrap=True, ha="right")
for size in ((4, 3), (8, 3), (2, 3)):
    fig.set_size_inches(*size)
    fig.canvas.draw()
    r = fig.canvas.get_renderer()
    print(size, [t._get_wrapped_text().count("\n") for t in (t1, t2, t3)],
          [round(t.get_window_extent(r).width, 1) for t in (t1, t2, t3)])
print(repr(t1._get_wrapped_text()[:60]))
fig2, ax = plt.subplots(figsize=(3, 3), dpi=72)
t4 = ax.text(0.1, 0.5, msg, wrap=True)
fig2.canvas.draw()
print(t4._get_wrapped_text().count("\n"), t4._get_wrap_line_width())
t4.set_wrap(False)
fig2.canvas.draw()
print(t4.get_wrap(), t4._get_wrapped_text().count("\n"))
fig.savefig("wrapped.png")
fig2.savefig("wrapped_axes.png")
t5 = fig2.text(0.5, 0.05, "averyveryverylongwordwithoutspaces " * 3, wrap=True, ha="center")
fig2.canvas.draw()
print(t5._get_wrapped_text().count("\n"), round(t5.get_window_extent().width, 1))
t1.set_fontsize(16)
fig.canvas.draw()
print(t1._get_wrapped_text().count("\n"))
