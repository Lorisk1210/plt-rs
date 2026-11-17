# LightSource shading of a terrain with each blend mode and vert_exag; hillshade statistics and shade_rgb on an RGB image.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.colors import LightSource

y, x = np.mgrid[-3:3:60j, -3:3:80j]
z = np.exp(-(x ** 2 + y ** 2)) * 3 + 0.5 * np.sin(2 * x)
ls = LightSource(azdeg=315, altdeg=35)
hs = ls.hillshade(z, vert_exag=2, dx=0.1, dy=0.1)
print(hs.shape, round(float(hs.min()), 4), round(float(hs.max()), 4), round(float(hs.mean()), 4))
for mode in ("hsv", "overlay", "soft"):
    rgb = ls.shade(z, cmap=plt.get_cmap("gist_earth"), blend_mode=mode, vert_exag=1.5)
    print(mode, rgb.shape, np.round(rgb[30, 40], 4).tolist(), round(float(rgb[..., :3].mean()), 4))
custom = ls.shade(z, cmap=plt.get_cmap("viridis"), blend_mode=lambda rgb, intensity: rgb * intensity)
print(np.round(custom[0, 0], 4).tolist())
base = plt.get_cmap("terrain")((z - z.min()) / np.ptp(z))[..., :3]
shaded = ls.shade_rgb(base, z, fraction=0.7)
print(shaded.shape, round(float(shaded.mean()), 4))
print(np.round(ls.direction, 4).tolist())
print(np.round(ls.blend_overlay(np.full((1, 1, 3), 0.25), np.full((1, 1, 1), 0.8)), 4).tolist())
print(np.round(ls.blend_soft_light(np.full((1, 1, 3), 0.25), np.full((1, 1, 1), 0.8)), 4).tolist())
fig, ax = plt.subplots(figsize=(3, 2))
ax.imshow(ls.shade(z, cmap=plt.get_cmap("copper")))
fig.savefig("shaded.png")
for alt in (10, 45, 80):
    h = LightSource(azdeg=90, altdeg=alt).hillshade(z, vert_exag=1)
    print(alt, round(float(h.mean()), 4), round(float(h.std()), 4))
print(np.round(ls.shade(z, cmap=plt.get_cmap("gray"), fraction=0.3)[0, 0], 4).tolist())
