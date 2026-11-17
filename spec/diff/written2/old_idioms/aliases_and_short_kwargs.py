# Short aliases everywhere (lw, ls, c, mec, mfc, ms, fc, ec, aa, fontsize vs size), and conflicts between alias and full name.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
ln, = ax.plot([0, 1], lw=3, ls="--", c="m", marker="o", mec="k", mfc="y", ms=9, mew=2, aa=False)
print(ln.get_linewidth(), ln.get_linestyle(), ln.get_color(), ln.get_markeredgecolor(), ln.get_markerfacecolor(),
      ln.get_markersize(), ln.get_markeredgewidth(), ln.get_antialiased())
p = ax.add_patch(plt.Rectangle((0, 0), 1, 1, fc="c", ec="r", lw=2, ls=":"))
print(p.get_facecolor(), p.get_edgecolor(), p.get_linewidth(), p.get_linestyle())
t = ax.text(0, 0, "t", size=14, family="serif", weight="bold", style="italic", va="top", ha="right", rotation=10)
print(t.get_fontsize(), t.get_fontfamily(), t.get_fontweight(), t.get_fontstyle(), t.get_va(), t.get_ha())
t.set(fontsize=20)
print(t.get_size())
for kw in [dict(lw=1, linewidth=2), dict(c="r", color="b"), dict(ls="-", linestyle="--")]:
    try:
        ax.plot([0, 1], **kw)
        print(sorted(kw), "accepted", ax.lines[-1].get_linewidth(), ax.lines[-1].get_color())
    except TypeError as e:
        print(sorted(kw), "TypeError", str(e)[:60])
sc = ax.scatter([0], [0], c="r", s=20, ec="k", lw=3)
print(sc.get_edgecolor().tolist(), sc.get_linewidths())
ln.set_lw(1.5)
ln.set_ls("-.")
ln.set_c("g")
print(ln.get_lw(), ln.get_ls(), ln.get_c())
print(type(ln).set_lw is type(ln).set_linewidth, type(ln).set_lw.__doc__[:30])
print(sorted(type(ln)._alias_to_prop)[:5])
fig.savefig("aliases.png")
