# Image with a line profile: draw a line on the image and plot the sampled intensities below.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:100, 0:150]
img = np.exp(-((x - 75) ** 2 + (y - 50) ** 2) / 400) + 0.3 * np.exp(-((x - 30) ** 2 + (y - 30) ** 2) / 100)
x0, y0, x1, y1 = 10, 20, 140, 80
n = 200
xs, ys = np.linspace(x0, x1, n), np.linspace(y0, y1, n)
profile = img[ys.round().astype(int), xs.round().astype(int)]
fig, (a, b) = plt.subplots(2, 1, figsize=(6, 6), gridspec_kw={"height_ratios": [2, 1]})
a.imshow(img, cmap="inferno")
a.plot([x0, x1], [y0, y1], "c-", lw=2)
b.plot(np.hypot(xs - x0, ys - y0), profile)
b.set_xlabel("distance along line [px]")
print(round(float(profile.max()), 4))
fig.savefig("profile.png")
