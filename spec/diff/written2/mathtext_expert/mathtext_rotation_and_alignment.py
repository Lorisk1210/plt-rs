# Rotated and aligned mathtext labels: window extents for every ha/va/rotation, including rotation_mode anchor.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 6))
ax.set_xlim(0, 4)
ax.set_ylim(0, 4)
expr = r"$\int_a^b f(x)\,dx$"
rows = []
for i, ha in enumerate(("left", "center", "right")):
    for j, va in enumerate(("top", "center", "baseline", "bottom", "center_baseline")):
        for rot in (0, 45, 90):
            t = ax.text(0.5 + i * 1.3, 0.4 + j * 0.75, expr, ha=ha, va=va, rotation=rot, fontsize=10,
                        rotation_mode="anchor" if rot == 45 else "default")
            rows.append((ha, va, rot, t))
fig.canvas.draw()
r = fig.canvas.get_renderer()
for ha, va, rot, t in rows:
    b = t.get_window_extent(r)
    print(ha, va, rot, [round(v, 1) for v in b.bounds])
fig.savefig("math_align.png")
anchor = [t for ha, va, rot, t in rows if rot == 45]
print(sorted({t.get_rotation_mode() for t in anchor}), len(anchor))
t0 = rows[0][3]
t0.set_rotation(30)
t0.set_text(r"$\sqrt{x}$ and text")
fig.canvas.draw()
print([round(v, 1) for v in t0.get_window_extent(r).bounds], t0.get_rotation())
tight = ax.get_tightbbox(r)
print([round(v, 1) for v in tight.bounds])
fig.savefig("math_align2.png")
