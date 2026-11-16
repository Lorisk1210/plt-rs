# Colour conversions: rgb_to_hsv / hsv_to_rgb to build a hue wheel and lighten colours.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors

hue = np.linspace(0, 1, 360)
sat = np.linspace(0, 1, 50)
H, S = np.meshgrid(hue, sat)
rgb = mcolors.hsv_to_rgb(np.dstack([H, S, np.ones_like(H)]))
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3))
a.imshow(rgb, aspect="auto", origin="lower", extent=[0, 360, 0, 1])
a.set_xlabel("hue")
a.set_ylabel("saturation")
base = mcolors.to_rgb("tab:blue")
hsv = mcolors.rgb_to_hsv(base)
for k, v in enumerate(np.linspace(0.3, 1.0, 6)):
    b.bar(k, 1, color=mcolors.hsv_to_rgb([hsv[0], hsv[1], v]))
print(np.round(hsv, 4))
fig.savefig("hsv.png")
