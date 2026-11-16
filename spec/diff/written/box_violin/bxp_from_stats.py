# Draw box plots from precomputed statistics with bxp, as when only summaries are available.
import matplotlib.pyplot as plt

stats = [
    dict(label="site A", med=12, q1=9, q3=15, whislo=4, whishi=21, fliers=[25, 27], mean=12.6),
    dict(label="site B", med=18, q1=14, q3=22, whislo=8, whishi=30, fliers=[], mean=18.4),
    dict(label="site C", med=10, q1=8, q3=13, whislo=6, whishi=17, fliers=[2, 22], mean=11.2),
]
fig, ax = plt.subplots(figsize=(5, 3.5))
art = ax.bxp(stats, showmeans=True, patch_artist=True)
for box in art["boxes"]:
    box.set_facecolor("honeydew")
ax.set_ylabel("PM10 [µg/m³]")
ax.set_title("Air quality summary")
print(len(art["fliers"]), [t.get_text() for t in ax.get_xticklabels()])
fig.savefig("bxp.png")
