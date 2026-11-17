# Text with characters missing from the first family: font fallback across a family list, measured and drawn to a buffer.
import io
import warnings
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

samples = ["plain ASCII", "Greek αβγ ΔΩ", "math ∑∫√∞ ≤ ≥", "arrows ← → ↔", "box ┌─┐ █", "accents éàüñ"]
fig, ax = plt.subplots(figsize=(6, 4))
ax.set_axis_off()
texts = []
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    for k, s in enumerate(samples):
        t = ax.text(0.02, 0.9 - 0.15 * k, s, fontsize=13, family=["cmr10", "STIXGeneral", "DejaVu Sans"])
        texts.append(t)
    fig.canvas.draw()
print(sorted({str(w.message)[:50] for w in caught}))
r = fig.canvas.get_renderer()
for t in texts:
    print(repr(t.get_text()), round(t.get_window_extent(r).width, 2), t.get_fontfamily())
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    t2 = ax.text(0.5, 0.05, "only DejaVu: ∑ ✓ ☃", family="DejaVu Sans")
    fig.canvas.draw()
print(len(caught), round(t2.get_window_extent(r).width, 2))
buf = io.BytesIO()
fig.savefig(buf, format="png")
print(buf.getvalue()[:8], len(buf.getvalue()) > 1000)
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    t3 = ax.text(0.5, 0.5, "中文", family="DejaVu Sans")
    fig.canvas.draw()
print(sorted({str(w.message)[:40] for w in caught}))
