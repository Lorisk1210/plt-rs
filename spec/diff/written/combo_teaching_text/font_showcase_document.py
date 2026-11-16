# Typography sheet: font families, weights and styles in a table-like grid, plus mathtext fontsets and a unicode row.
import matplotlib.pyplot as plt

families = ["DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono", "sans-serif", "serif", "monospace"]
fig, ax = plt.subplots(figsize=(10, 6))
ax.axis("off")
for i, fam in enumerate(families):
    for j, (weight, style) in enumerate([("normal", "normal"), ("bold", "normal"), ("normal", "italic"), ("bold", "oblique")]):
        ax.text(0.02 + j * 0.24, 0.95 - i * 0.1, f"{fam} {weight}/{style}", family=fam, weight=weight, style=style, fontsize=9,
                transform=ax.transAxes)
for k, fs in enumerate(["dejavusans", "cm", "stix"]):
    with plt.rc_context({"mathtext.fontset": fs}):
        ax.text(0.02 + k * 0.32, 0.25, rf"{fs}: $\int_0^\infty e^{{-x^2}} dx = \frac{{\sqrt{{\pi}}}}{{2}}$", fontsize=11, transform=ax.transAxes)
ax.text(0.02, 0.1, "Unicode: αβγ ∑∫√∞ ≤≥≠ °C µm Å €£¥ →←↔", fontsize=12, transform=ax.transAxes)
fig.canvas.draw()
print(len(ax.texts), ax.texts[1].get_fontweight(), ax.texts[2].get_fontstyle())
fig.savefig("typography.png", dpi=50)
