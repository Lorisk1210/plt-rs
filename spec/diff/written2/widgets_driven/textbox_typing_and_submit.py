# TextBox typed into with key events after a click, with backspace, arrows and enter; on_text_change/on_submit logs.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, KeyEvent
from matplotlib.widgets import TextBox

fig = plt.figure(figsize=(4, 2), dpi=100)
tax = fig.add_axes([0.2, 0.4, 0.6, 0.25])
box = TextBox(tax, "f(x) = ", initial="x**2", textalignment="left")
fig.canvas.draw()
changes, submits = [], []
box.on_text_change(changes.append)
box.on_submit(submits.append)
bb = tax.get_window_extent()
MouseEvent("button_press_event", fig.canvas, bb.x0 + 5, bb.y0 + 5, 1)._process()
MouseEvent("button_release_event", fig.canvas, bb.x0 + 5, bb.y0 + 5, 1)._process()
print(box.capturekeystrokes, box.cursor_index)
for key in ["right", "right", "right", "right", "+", "1", "backspace", "2", "left", "delete", "home", "-", "end", " ", "enter"]:
    KeyEvent("key_press_event", fig.canvas, key, bb.x0 + 5, bb.y0 + 5)._process()
print(changes)
print(submits, repr(box.text), box.capturekeystrokes)
box.set_val("sin(x)")
print(changes[-1], submits[-1], box.text)
box.begin_typing()
KeyEvent("key_press_event", fig.canvas, "!", 0, 0)._process()
box.stop_typing()
print(box.text, submits[-1])
box.set_active(False)
KeyEvent("key_press_event", fig.canvas, "z", 0, 0)._process()
print(box.text, box.get_active())
fig.savefig("textbox.png")
