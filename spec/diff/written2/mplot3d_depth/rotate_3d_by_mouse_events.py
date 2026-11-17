# Rotate, pan and zoom an Axes3D with synthetic mouse drags (left, middle, right buttons) and read elev/azim/limits afterwards.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig = plt.figure(figsize=(4, 4), dpi=100)
ax = fig.add_subplot(projection="3d")
ax.scatter(*np.indices((3, 3, 3)).reshape(3, -1))
fig.canvas.draw()


def state():
    return (round(ax.elev, 3), round(ax.azim, 3), round(ax.roll, 3),
            [np.round(f(), 3).tolist() for f in (ax.get_xlim, ax.get_ylim, ax.get_zlim)])


def drag(button, dx, dy, key=None):
    x0, y0 = 200, 200
    MouseEvent("button_press_event", fig.canvas, x0, y0, button, key=key)._process()
    for k in (1, 2):
        MouseEvent("motion_notify_event", fig.canvas, x0 + dx * k / 2, y0 + dy * k / 2, button, key=key)._process()
    MouseEvent("button_release_event", fig.canvas, x0 + dx, y0 + dy, button, key=key)._process()


print("start", state())
drag(1, 40, 0)
print("rotate x", state())
drag(1, 0, 30)
print("rotate y", state())
drag(2, 20, 20)
print("pan", state())
drag(3, 0, 40)
print("zoom", state())
ax.mouse_init(rotate_btn=3, pan_btn=1, zoom_btn=2)
drag(3, -40, 0)
print("rotate via 3", state())
ax.disable_mouse_rotation()
drag(1, 50, 50)
print("disabled", state())
with plt.rc_context({"axes3d.mouserotationstyle": "trackball"}):
    ax.mouse_init()
    drag(1, 30, 10)
    print("trackball", state())
fig.savefig("rotated.png")
