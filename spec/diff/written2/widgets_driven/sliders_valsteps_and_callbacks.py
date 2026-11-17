# Slider and RangeSlider with valstep arrays, slidermin/slidermax links, valfmt and on_changed disconnects; set_val sequences.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider, RangeSlider

fig = plt.figure(figsize=(5, 3))
a1 = fig.add_axes([0.2, 0.75, 0.6, 0.08])
a2 = fig.add_axes([0.2, 0.55, 0.6, 0.08])
a3 = fig.add_axes([0.2, 0.35, 0.6, 0.08])
a4 = fig.add_axes([0.2, 0.15, 0.6, 0.08])
lo = Slider(a1, "lo", 0, 10, valinit=2, valstep=0.5)
hi = Slider(a2, "hi", 0, 10, valinit=8, slidermin=lo, valfmt="%1.3f")
steps = Slider(a3, "steps", 0, 100, valinit=10, valstep=[0, 1, 2, 5, 10, 20, 50, 100])
rng = RangeSlider(a4, "range", -5, 5, valinit=(-1, 2), valstep=0.25)
log = []
cid = lo.on_changed(lambda v: log.append(("lo", v)))
hi.on_changed(lambda v: log.append(("hi", v)))
steps.on_changed(lambda v: log.append(("steps", v)))
rng.on_changed(lambda v: log.append(("rng", tuple(round(x, 3) for x in v))))
for v in (3.3, 3.74, 11, -2):
    lo.set_val(v)
for v in (1, 9.12345):
    hi.set_val(v)
for v in (3, 7.5, 33, 99):
    steps.set_val(v)
rng.set_val((-4.9, 4.9))
rng.set_min(0.1)
rng.set_max(-3)
for entry in log:
    print(entry)
print(lo.val, hi.val, hi.valtext.get_text(), steps.val, rng.val, rng.valtext.get_text())
lo.disconnect(cid)
lo.set_val(5)
print(log[-1], lo.val)
lo.reset()
rng.reset()
print(lo.val, rng.val, lo.valinit)
lo.eventson = False
lo.set_val(7)
print(log[-1], lo.val)
fig.savefig("sliders.png")
