# PathPatchEffect, Stroke and withSimplePatchShadow on patches, texts, annotations and tick labels; rcParams path.effects default.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.patheffects as pe
from matplotlib.patches import Circle, FancyBboxPatch


fig, ax = plt.subplots(figsize=(5, 4))
c = ax.add_patch(Circle((0.3, 0.5), 0.2, fc="C0",
                        path_effects=[pe.withSimplePatchShadow(offset=(4, -4), shadow_rgbFace="0.3", alpha=0.6)]))
b = ax.add_patch(FancyBboxPatch((0.6, 0.3), 0.3, 0.3, boxstyle="round,pad=0.05", fc="w",
                                path_effects=[pe.PathPatchEffect(offset=(3, -3), hatch="xx", facecolor="gray"),
                                              pe.PathPatchEffect(edgecolor="k", linewidth=1.2, facecolor="w")]))
txt = ax.text(0.5, 0.9, "STROKED", ha="center", fontsize=20, color="w",
              path_effects=[pe.Stroke(linewidth=3, foreground="k"), pe.Normal()])
ann = ax.annotate("note", (0.3, 0.5), (0.1, 0.1), arrowprops=dict(arrowstyle="->", lw=2,
                  path_effects=[pe.Stroke(linewidth=5, foreground="y"), pe.Normal()]))
for lab in ax.get_xticklabels():
    lab.set_path_effects([pe.withStroke(linewidth=2, foreground="orange")])
fig.canvas.draw()
print([type(e).__name__ for e in c.get_path_effects()])
print([type(e).__name__ for e in b.get_path_effects()])
print([type(e).__name__ for e in ann.arrow_patch.get_path_effects()])
print(len(ax.get_xticklabels()), [type(e).__name__ for e in ax.get_xticklabels()[0].get_path_effects()])
fig.savefig("patch_effects.png")
with plt.rc_context({"path.effects": [pe.withStroke(linewidth=3, foreground="pink")]}):
    fig2, ax2 = plt.subplots(figsize=(3, 2))
    ln, = ax2.plot([0, 1], [1, 0])
    t2 = ax2.set_title("rc effects")
    print([type(e).__name__ for e in ln.get_path_effects()], [type(e).__name__ for e in t2.get_path_effects()])
    fig2.savefig("rc_effects.png")
print([type(e).__name__ for e in txt.get_path_effects()], plt.rcParams["path.effects"])
