# Subclass TimedAnimation: a counter animation implementing new_frame_seq and _draw_frame; frames stepped with _step.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation


class Countdown(animation.TimedAnimation):
    def __init__(self, fig, start, **kw):
        self.start_value = start
        self.text = fig.text(0.5, 0.5, "", ha="center", fontsize=20)
        self.drawn = []
        super().__init__(fig, interval=100, repeat=False, **kw)

    def new_frame_seq(self):
        return iter(range(self.start_value, -1, -1))

    def _draw_frame(self, framedata):
        self.text.set_text(str(framedata) if framedata else "go!")
        self.drawn.append(framedata)
        self._drawn_artists = [self.text]

    def _init_draw(self):
        self.text.set_text("ready")
        self._drawn_artists = [self.text]


fig = plt.figure(figsize=(2, 1), dpi=40)
anim = Countdown(fig, 3)
print(anim.text.get_text(), anim.drawn)
anim._init_draw()
while anim._step():
    pass
print(anim.drawn, anim.text.get_text())
anim2 = Countdown(fig, 2)
anim2.save("countdown.gif", writer="pillow", fps=2)
print(anim2.drawn, anim2.text.get_text())
from PIL import Image
with Image.open("countdown.gif") as im:
    print(im.n_frames)
print(isinstance(anim, animation.Animation), anim._interval, anim._repeat)
