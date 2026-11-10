"""Artists the mouse acts on, and drawing without a full draw, for
``interaction.py``: picking (every kind of ``picker``, every kind of
artist, overlapping artists), dragging legends, annotations and offset
boxes, blitting onto the canvas, and animations stepped, saved and
written as HTML.

What a blit leaves on the canvas is compared as a picture: the canvas'
buffer is shown on a figure of its own (``figimage``), which the
tester compares at the end like every figure; so are the frames of the
GIF and APNG files the animations write.
"""

from __future__ import annotations

import shutil
from typing import Iterator

#: A scene of every kind of pickable artist.
PICKABLE = [
    "fig, ax = plt.subplots(figsize=(4, 3))",
    "def on_pick(event):\n"
    "    a = event.artist\n"
    "    ind = getattr(event, 'ind', None)\n"
    "    m = event.mouseevent\n"
    "    show('pick', type(a).__name__, a.get_label(), None if ind is None else list(ind), m.name, m.button, "
    "m.xdata, m.ydata, sorted(k for k in vars(event) if k not in ('artist', 'mouseevent', 'canvas', 'guiEvent')))",
    "pick_cid = fig.canvas.mpl_connect('pick_event', on_pick)",
    "line = ax.plot(x, y, 'o-', picker=True, pickradius=8, label='line')[0]",
    "markers = ax.plot(x, -y, 's', picker=4, label='markers')[0]",
    "dots = ax.scatter(x, y / 2, s=80, picker=True, label='dots')",
    "rect = ax.add_patch(patches.Rectangle((1, -0.9), 1.5, 0.4, picker=True, label='rect'))",
    "word = ax.text(4, 0.8, 'pick me', picker=True, label='word')",
    "picture = ax.imshow(z, extent=(5, 6, -1, -0.5), aspect='auto', picker=True, label='picture')",
    "note = ax.annotate('note', xy=(3, 0), xytext=(2.5, -0.6), picker=5, arrowprops=dict(arrowstyle='->'))",
    "bars = ax.bar([0.5], [0.3], width=0.4, picker=True, label='bars')",
    "lines = ax.vlines([5.5], -0.2, 0.6, picker=3, label='vlines')",
    "ax.set_xlim(-0.5, 6.5)",
    "fig.canvas.draw()",
    "def click(xy, button=MouseButton.LEFT):\n"
    "    where = tuple(round(float(v)) for v in ax.transData.transform(xy))\n"
    "    mouse('button_press_event', where, button)\n"
    "    mouse('button_release_event', where, button)",
    "def at_text(t):\n"
    "    box = t.get_window_extent()\n"
    "    return ax.transData.inverted().transform(((box.x0 + box.x1) / 2, (box.y0 + box.y1) / 2))",
]


def _picking() -> Iterator[str]:
    for point in ("(x[1], y[1])", "((x[1] + x[2]) / 2, (y[1] + y[2]) / 2)", "(x[2], -y[2])", "(x[3], y[3] / 2)",
                  "(1.7, -0.7)", "at_text(word)", "(5.5, -0.75)", "at_text(note)", "(3, 0)", "(0.5, 0.1)",
                  "(5.5, 0.3)", "(0, 0)", "(6.2, 0.9)"):
        yield f"click({point})"
    yield "click((x[1], y[1]), MouseButton.RIGHT)"
    yield "[(a.get_label(), a.get_picker(), a.pickable()) for a in (line, markers, dots, rect, word, picture, note)]"
    yield "line.get_pickradius(), markers.get_pickradius(), dots.get_pickradius()"
    yield "line.set_pickradius(1)"
    yield "dots.set_pickradius(15)"
    yield "click(((x[1] + x[2]) / 2, (y[1] + y[2]) / 2 + 0.05))"
    yield "click((x[3] + 0.15, y[3] / 2))"
    yield "def only_right(artist, mouseevent):\n" \
          "    hit = mouseevent.xdata is not None and mouseevent.xdata > 3\n" \
          "    return (hit, {'side': 'right'}) if hit else (False, {})"
    yield "markers.set_picker(only_right)"
    yield "click((x[2], -y[2]))"
    yield "click((x[5], -y[5]))"
    yield "line.set_visible(False)"
    yield "click((x[1], y[1]))"
    yield "line.set_visible(True)"
    yield "dots.set_picker(None)"
    yield "click((x[3], y[3] / 2))"
    yield "ax.set_picker(True)"
    yield "click((6.2, -0.95))"
    yield "probe = MouseEvent('button_press_event', fig.canvas, *ax.transData.transform((x[1], y[1])), 1)"
    yield "line.contains(probe), dots.contains(probe), rect.contains(probe), word.contains(probe)"
    yield "picture.contains(probe), note.contains(probe), bars.patches[0].contains(probe)"
    yield "ax.pick(probe)"
    yield "fig.pick(probe)"
    yield "fig.canvas.mpl_disconnect(pick_cid)"
    yield "click((x[1], y[1]))"


def _legend_drag() -> Iterator[str]:
    yield "fig, ax = plt.subplots(figsize=(4, 3))"
    yield "ax.plot(x, y, label='sine')"
    yield "ax.plot(x, -y, label='minus')"
    yield "leg = ax.legend(loc='upper right')"
    yield "fig.canvas.draw()"
    yield "def centre(artist):\n" \
          "    box = artist.get_window_extent()\n" \
          "    return round((box.x0 + box.x1) / 2), round((box.y0 + box.y1) / 2)"
    yield "def legend_state():\n" \
          "    return (leg._loc, leg.get_bbox_to_anchor().bounds, leg.get_window_extent().bounds, " \
          "leg.get_draggable())"
    yield "legend_state()"
    for update, blit in (("loc", False), ("bbox", False), ("loc", True)):
        yield f"dragger = leg.set_draggable(True, use_blit={blit}, update={update!r})"
        yield "type(dragger).__name__, leg.get_draggable()"
        yield "start = centre(leg)"
        yield "drag(start, (start[0] - 60, start[1] - 40), steps=3)"
        yield "fig.canvas.draw()"
        yield "legend_state()"
        yield "drag((start[0] - 60, start[1] - 40), (start[0] - 200, start[1] + 400), steps=2)"
        yield "legend_state()"
    yield "drag(centre(leg), (10, 10), MouseButton.RIGHT)"
    yield "legend_state()"
    yield "leg.set_draggable(False)"
    yield "drag(centre(leg), (100, 100))"
    yield "legend_state()"
    yield "leg.set_draggable(True, update='nope')"
    yield "figleg = fig.legend(loc='lower left', draggable=True)"
    yield "fig.canvas.draw()"
    yield "drag(centre(figleg), (centre(figleg)[0] + 50, centre(figleg)[1] + 30))"
    yield "figleg._loc, figleg.get_window_extent().bounds"


def _annotation_drag() -> Iterator[str]:
    yield "from matplotlib.offsetbox import AnchoredText, AnnotationBbox, DraggableOffsetBox, OffsetImage, TextArea"
    yield "fig, ax = plt.subplots(figsize=(4, 3))"
    yield "ax.plot(x, y)"
    yield "ann = ax.annotate('peak', xy=(x[2], y[2]), xytext=(2, 0.4), arrowprops=dict(arrowstyle='->'))"
    yield "points = ax.annotate('pts', xy=(x[4], y[4]), xytext=(20, 20), textcoords='offset points')"
    yield "anchored = ax.add_artist(AnchoredText('anchored', loc='lower left'))"
    yield "box = ax.add_artist(AnnotationBbox(OffsetImage(z, zoom=4), (x[1], y[1]), xybox=(4.5, 0.5)))"
    yield "frame = ax.add_artist(AnnotationBbox(TextArea('frame'), (1, -0.5), xybox=(30, -30), " \
          "boxcoords='offset points'))"
    yield "fig.canvas.draw()"
    yield "def centre(artist):\n" \
          "    box = artist.get_window_extent()\n" \
          "    return round((box.x0 + box.x1) / 2), round((box.y0 + box.y1) / 2)"
    yield "def state():\n" \
          "    fig.canvas.draw()\n" \
          "    return (ann.xyann, ann.get_position(), points.xyann, box.xybox, frame.xybox, " \
          "anchored.get_window_extent().bounds, box.get_window_extent().bounds)"
    yield "state()"
    yield "d1 = ann.draggable()"
    yield "d2 = points.draggable(True, use_blit=True)"
    yield "d3 = box.draggable()"
    yield "d4 = DraggableOffsetBox(anchored, anchored)"
    yield "d5 = frame.draggable(use_blit=True)"
    yield "[type(d).__name__ for d in (d1, d2, d3, d4, d5)], ann.draggable() is None"
    for artist in ("ann", "points", "box", "anchored", "frame"):
        yield f"start = centre({artist})"
        yield "drag(start, (start[0] + 35, start[1] - 25), steps=3)"
        yield "state()"
    yield "ann.draggable(False)"
    yield "start = centre(ann)"
    yield "drag(start, (start[0] + 35, start[1] - 25))"
    yield "state()"
    yield "d4.disconnect()"
    yield "start = centre(anchored)"
    yield "drag(start, (start[0] + 35, start[1] + 25))"
    yield "state()"
    yield "box.remove()"
    yield "drag(centre(points), (5, 5))"
    yield "points.xyann"


def _blitting() -> Iterator[str]:
    yield "fig, ax = plt.subplots(figsize=(4, 3))"
    yield "ax.set_ylim(-1.5, 1.5)"
    yield "line = ax.plot(x, y, animated=True)[0]"
    yield "label = ax.text(0.05, 0.9, 'frame 0', transform=ax.transAxes, animated=True)"
    yield "still = ax.plot(x, -y, 'k:')[0]"
    yield "fig.canvas.supports_blit, line.get_animated(), still.get_animated()"
    yield "fig.canvas.draw()"
    yield "def snapshot():\n    return np.asarray(fig.canvas.buffer_rgba()).copy()"
    yield "def shown(buffer):\n" \
          "    f = plt.figure(figsize=(buffer.shape[1] / 100, buffer.shape[0] / 100))\n" \
          "    f.figimage(buffer)\n" \
          "    return buffer.shape"
    yield "base = snapshot()"
    yield "shown(base)"
    yield "background = fig.canvas.copy_from_bbox(fig.bbox)"
    yield "type(background).__name__, background.get_extents()"
    yield "ax.draw_artist(line)"
    yield "ax.draw_artist(label)"
    yield "drawn = snapshot()"
    yield "np.array_equal(base, drawn), shown(drawn)"
    yield "fig.canvas.blit(fig.bbox)"
    yield "fig.canvas.restore_region(background)"
    yield "np.array_equal(snapshot(), base)"
    yield "region = fig.canvas.copy_from_bbox(ax.bbox)"
    yield "region.get_extents()"
    for i in range(3):
        yield "fig.canvas.restore_region(background)"
        yield f"line.set_ydata(np.sin(x + {i}))"
        yield f"label.set_text('frame {i + 1}')"
        yield "ax.draw_artist(line)"
        yield "ax.draw_artist(label)"
        yield "fig.canvas.blit(ax.bbox)"
        yield "np.array_equal(snapshot(), base), np.array_equal(snapshot(), drawn)"
    yield "shown(snapshot())"
    yield "fig.canvas.restore_region(region, bbox=(80, 60, 200, 160), xy=(10, 10))"
    yield "shown(snapshot())"
    yield "fig.canvas.restore_region(region, xy=(150, 100))"
    yield "shown(snapshot())"
    yield "fig.canvas.restore_region(background)"
    yield "np.array_equal(snapshot(), base)"
    yield "corner = fig.canvas.copy_from_bbox(transforms.Bbox([[-20, -20], [50, 40]]))"
    yield "corner.get_extents()"
    yield "fig.canvas.blit()"
    yield "fig.canvas.draw()"
    yield "np.array_equal(snapshot(), base)"
    yield "class BlitManager:\n" \
          "    def __init__(self, canvas, animated_artists=()):\n" \
          "        self.canvas = canvas\n" \
          "        self._bg = None\n" \
          "        self._artists = []\n" \
          "        for a in animated_artists:\n" \
          "            self.add_artist(a)\n" \
          "        self.cid = canvas.mpl_connect('draw_event', self.on_draw)\n" \
          "    def on_draw(self, event):\n" \
          "        cv = self.canvas\n" \
          "        if event is not None:\n" \
          "            if event.canvas != cv:\n" \
          "                raise RuntimeError\n" \
          "        self._bg = cv.copy_from_bbox(cv.figure.bbox)\n" \
          "        self._draw_animated()\n" \
          "    def add_artist(self, art):\n" \
          "        if art.figure != self.canvas.figure:\n" \
          "            raise RuntimeError\n" \
          "        art.set_animated(True)\n" \
          "        self._artists.append(art)\n" \
          "    def _draw_animated(self):\n" \
          "        fig = self.canvas.figure\n" \
          "        for a in self._artists:\n" \
          "            fig.draw_artist(a)\n" \
          "    def update(self):\n" \
          "        cv = self.canvas\n" \
          "        fig = cv.figure\n" \
          "        if self._bg is None:\n" \
          "            self.on_draw(None)\n" \
          "        else:\n" \
          "            cv.restore_region(self._bg)\n" \
          "            self._draw_animated()\n" \
          "            cv.blit(fig.bbox)\n" \
          "        cv.flush_events()"
    yield "counter = ax.annotate('0', (0, 1), xycoords='axes fraction', xytext=(10, -10), " \
          "textcoords='offset points', ha='left', va='top')"
    yield "manager = BlitManager(fig.canvas, [line, counter])"
    yield "fig.canvas.draw()"
    yield "manager._bg is not None"
    for j in range(4):
        yield f"line.set_ydata(np.sin(x + {j} / 3))"
        yield f"counter.set_text('frames: {j}')"
        yield "manager.update()"
    yield "shown(snapshot())"
    yield "fig.canvas.flush_events()"


def _animation(blit: bool) -> Iterator[str]:
    yield "from matplotlib import animation"
    yield "from PIL import Image, ImageSequence"
    yield "import re"
    yield "fig, ax = plt.subplots(figsize=(3, 2), dpi=50)"
    yield "ax.set_ylim(-1.5, 1.5)"
    yield "line = ax.plot(x, y)[0]"
    yield "def update(i):\n    print('update', i)\n    line.set_ydata(np.sin(x + i / 2))\n    return [line]"
    yield "def init():\n    print('init')\n    line.set_ydata(np.zeros_like(x))\n    return [line]"
    yield f"anim = animation.FuncAnimation(fig, update, frames=3, init_func=init, blit={blit}, interval=20, " \
          "repeat=False)"
    yield "type(anim.event_source).__name__, anim.event_source.interval, anim._repeat, anim._blit"
    yield "fig.canvas.draw()"
    yield "line.get_ydata()"
    yield "anim._step()"
    yield "anim.pause()"
    yield "anim.event_source.callbacks, anim._blit"
    yield "anim.resume()"
    yield "line.get_ydata()"
    for _ in range(3):
        yield "anim._step()"
        yield "line.get_ydata()"
    yield "anim.event_source"
    yield "anim.pause()"
    yield "def frames(path):\n" \
          "    image = Image.open(path)\n" \
          "    shots = [np.asarray(f.convert('RGBA')) for f in ImageSequence.Iterator(image)]\n" \
          "    shown = plt.figure(figsize=(image.size[0] * len(shots) / 100, image.size[1] / 100))\n" \
          "    for k, shot in enumerate(shots):\n" \
          "        shown.figimage(shot, xo=k * image.size[0])\n" \
          "    return image.format, image.size, len(shots), image.info.get('duration'), image.info.get('loop')"
    yield "anim.save('frames.gif', writer='pillow', fps=5, progress_callback=lambda i, n: print('progress', i, n))"
    yield "frames('frames.gif')"
    yield "anim.save('frames.png', writer=animation.PillowWriter(fps=4), dpi=40, savefig_kwargs={'facecolor': 'y'})"
    yield "frames('frames.png')"
    yield "counting = animation.FuncAnimation(fig, update, frames=None, save_count=3, blit=" \
          f"{blit}, cache_frame_data=False)"
    yield "listed = animation.FuncAnimation(fig, update, frames=[5, 1, 3], repeat=True, repeat_delay=40)"
    yield "generated = animation.FuncAnimation(fig, update, frames=(i * 2 for i in range(2)), save_count=2)"
    yield "for each in (counting, listed, generated):\n    print(type(each.event_source).__name__, each._save_count)"
    yield "fig.canvas.draw()"
    yield "[listed._step() for _ in range(5)], listed.event_source.interval"
    yield "[generated._step() for _ in range(3)]"
    yield "counting.save('counting.gif', writer='pillow', fps=10, extra_anim=[listed])"
    yield "frames('counting.gif')"
    yield "def scrubbed(html):\n" \
          "    return re.sub(r'[0-9a-f]{32}', 'ID', re.sub(r'data:image/png;base64,[A-Za-z0-9+/=\\\\\\n]*', 'IMAGE', " \
          "html))"
    yield "scrubbed(anim.to_jshtml(fps=4, embed_frames=True, default_mode='reflect'))"
    yield "anim.save('frames.html', writer=animation.HTMLWriter(fps=4, embed_frames=False, default_mode='once'))"
    yield "scrubbed(open('frames.html').read())"
    yield "import os"
    yield "sorted(os.listdir('frames_frames'))"
    yield "anim.save('embedded.html', writer=animation.HTMLWriter(fps=3, embed_frames=True, embed_limit=50))"
    yield "scrubbed(open('embedded.html').read())"
    yield "anim._repr_html_() is None"
    yield "matplotlib.rcParams['animation.html'] = 'jshtml'"
    yield "scrubbed(anim._repr_html_())"
    yield "artists = [[ax.plot(x, np.cos(x + i), 'r')[0], ax.text(0.5, 0.5, str(i))] for i in range(3)]"
    yield f"flip = animation.ArtistAnimation(fig, artists, interval=30, repeat_delay=50, blit={blit})"
    yield "fig.canvas.draw()"
    yield "[[a.get_visible() for a in frame] for frame in artists]"
    yield "flip._step()"
    yield "[[a.get_visible() for a in frame] for frame in artists]"
    yield "flip.save('flip.gif', writer='pillow', fps=3)"
    yield "frames('flip.gif')"
    yield "sorted(animation.writers.list())"
    if shutil.which("ffmpeg"):
        yield "scrubbed(anim.to_html5_video())"


def programs() -> Iterator[tuple[str, list[str]]]:
    """The picking, dragging, blitting and animation programs, without
    the header."""
    yield "picking_artists", PICKABLE + list(_picking())
    yield "draggable_legend", list(_legend_drag())
    yield "draggable_annotations", list(_annotation_drag())
    yield "blitting_canvas", list(_blitting())
    yield "animation_blit", list(_animation(True))
    yield "animation_noblit", list(_animation(False))
