# Which rcParams act on artists already made and which only on new ones: change after creation, draw, and compare.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ln, = ax.plot([0, 1], [0, 1])
txt = ax.set_title("before")
leg = ax.legend(["l"])
before = (ln.get_linewidth(), txt.get_fontsize(), ax.get_facecolor(), ax.xaxis.get_ticklabels()[0].get_fontsize(),
          leg.get_frame().get_alpha(), ax.xaxis.get_major_ticks()[0].tick1line.get_markersize())
plt.rcParams.update({"lines.linewidth": 6, "axes.titlesize": 20, "axes.facecolor": "yellow", "xtick.labelsize": 4,
                     "legend.framealpha": 0.1, "xtick.major.size": 12, "axes.grid": True, "image.cmap": "gray",
                     "savefig.facecolor": "red", "path.sketch": None})
fig.canvas.draw()
after = (ln.get_linewidth(), txt.get_fontsize(), ax.get_facecolor(), ax.xaxis.get_ticklabels()[0].get_fontsize(),
         leg.get_frame().get_alpha(), ax.xaxis.get_major_ticks()[0].tick1line.get_markersize())
print(before)
print(after)
print(ax.xaxis.get_gridlines()[0].get_visible())
ax.set_xlim(0, 2)
fig.canvas.draw()
print(ax.xaxis.get_ticklabels()[-1].get_fontsize(), ax.xaxis.get_major_ticks()[-1].tick1line.get_markersize())
ln2, = ax.plot([0, 1], [1, 0])
im = ax.imshow([[0, 1]], extent=(0, 1, 0, 1))
print(ln2.get_linewidth(), im.get_cmap().name)
fig.savefig("rc_after.png")
import PIL.Image
with PIL.Image.open("rc_after.png") as png:
    print(png.convert("RGB").getpixel((1, 1)))
fig2, ax2 = plt.subplots(figsize=(4, 3))
print(ax2.get_facecolor(), ax2.xaxis.get_gridlines()[0].get_visible())
plt.rcdefaults()
fig.savefig("rc_reset.png")
with PIL.Image.open("rc_reset.png") as png:
    print(png.convert("RGB").getpixel((1, 1)))
