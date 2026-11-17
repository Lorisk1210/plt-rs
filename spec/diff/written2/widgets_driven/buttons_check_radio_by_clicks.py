# CheckButtons, RadioButtons and Button clicked by synthetic events at their label positions; states and callbacks.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.widgets import Button, CheckButtons, RadioButtons

fig = plt.figure(figsize=(5, 4), dpi=100)
check = CheckButtons(fig.add_axes([0.05, 0.5, 0.3, 0.4]), ["grid", "legend", "title"], [True, False, False])
radio = RadioButtons(fig.add_axes([0.4, 0.5, 0.3, 0.4]), ["red", "green", "blue"], active=1)
button = Button(fig.add_axes([0.75, 0.6, 0.2, 0.1]), "Apply", color="0.9", hovercolor="0.7")
fig.canvas.draw()
log = []
check.on_clicked(lambda label: log.append(("check", label)))
radio.on_clicked(lambda label: log.append(("radio", label)))
button.on_clicked(lambda event: log.append(("button", event.button)))


def click_text(text, button=1):
    bb = text.get_window_extent()
    x, y = bb.x0 + bb.width / 2, bb.y0 + bb.height / 2
    MouseEvent("button_press_event", fig.canvas, x, y, button)._process()
    MouseEvent("button_release_event", fig.canvas, x, y, button)._process()


click_text(check.labels[1])
click_text(check.labels[0])
click_text(radio.labels[2])
click_text(radio.labels[2])
click_text(button.label)
click_text(button.label, button=3)
print(log)
print(check.get_status(), check.get_checked_labels(), radio.value_selected, radio.index_selected)
check.set_active(2)
radio.set_active(0)
print(check.get_status(), radio.value_selected, log[-2:])
check.clear()
print(check.get_status())
radio.set_label_props(dict(color=["r", "g", "b"]))
print([t.get_color() for t in radio.labels])
fig.savefig("buttons.png")
