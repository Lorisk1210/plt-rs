"""The gallery's text, labels and annotations section
(``galleries/examples/text_labels_and_annotations``), one case per figure
(``<example>_<n>`` for examples drawing several).

The examples run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng`` and the Grace Hopper photograph from a
synthetic RGB image of its size. ``draggable()`` calls are dropped (they
only matter interactively). Examples needing usetex (``tex_demo``,
``usetex_baseline_test``, ``usetex_fonteffects``), fonts matplotlib does not
bundle (the Nimbus Sans and FreeMono figures of ``font_family_rc``), a
custom legend handler class (the last ``legend_demo`` figure) or a custom
image patch class (``demo_text_path``) are not ported."""

import inspect
import itertools
import os
import re
from io import BytesIO
from pathlib import Path

import matplotlib as mpl
import matplotlib.font_manager as fm
import numpy as np
from cycler import cycler
from matplotlib import lines as mlines
from matplotlib import text as mtext
from matplotlib import transforms as mtransforms
from matplotlib.figure import Figure
from matplotlib.font_manager import FontProperties
from matplotlib.ft2font import FT2Font
from matplotlib.legend_handler import HandlerTuple
from matplotlib.offsetbox import AnnotationBbox, DrawingArea, OffsetImage, TextArea
from matplotlib.patches import (Annulus, Arc, ArrowStyle, Circle, ConnectionPatch, Ellipse,
                                FancyArrowPatch, Patch, Rectangle)
from matplotlib.text import OffsetFrom
from matplotlib.transforms import Bbox, IdentityTransform, ScaledTranslation, TransformedBbox
from PIL import Image

from spec_support import case, rng

G = "gallery_text"


# --- accented_text ------------------------------------------------------------


@case(f"{G}/accented_text_1")
def _(fig):
    ax = fig.subplots()
    ax.plot(range(10))
    ax.set_title(r'$\ddot{o}\acute{e}\grave{e}\hat{O}'
                 r'\breve{i}\bar{A}\tilde{n}\vec{q}$', fontsize=20)
    ax.set_xlabel(r"""$\"o\ddot o \'e\`e\~n\.x\^y$""", fontsize=20)
    ax.text(4, 0.5, r"$F=m\ddot{x}$")
    fig.tight_layout()


@case(f"{G}/accented_text_2")
def _(fig):
    ax = fig.subplots()
    ax.set_title("GISCARD CHAHUTÉ À L'ASSEMBLÉE")
    ax.set_xlabel("LE COUP DE DÉ DE DE GAULLE")
    ax.set_ylabel('André was here!')
    ax.text(0.2, 0.8, 'Institut für Festkörperphysik', rotation=45)
    ax.text(0.4, 0.2, 'AVA (check kerning)')


# --- angle_annotation ---------------------------------------------------------


class AngleAnnotation(Arc):
    """The example's arc between two vectors that is circular on screen:
    centre, angles and size are recomputed in pixels at every draw."""

    def __init__(self, xy, p1, p2, size=75, unit="points", ax=None,
                 text="", textposition="inside", text_kw=None, **kwargs):
        self.ax = ax
        self._xydata = xy
        self.vec1 = p1
        self.vec2 = p2
        self.size = size
        self.unit = unit
        self.textposition = textposition
        super().__init__(self._xydata, size, size, angle=0.0,
                         theta1=self.theta1, theta2=self.theta2, **kwargs)
        self.set_transform(IdentityTransform())
        self.ax.add_patch(self)
        self.kw = dict(ha="center", va="center", xycoords=IdentityTransform(),
                       xytext=(0, 0), textcoords="offset points", annotation_clip=True)
        self.kw.update(text_kw or {})
        self.text = self.ax.annotate(text, xy=self._center, **self.kw)

    def get_size(self):
        factor = 1.
        if self.unit == "points":
            factor = self.ax.figure.dpi / 72.
        elif self.unit[:4] == "axes":
            b = TransformedBbox(Bbox.unit(), self.ax.transAxes)
            dic = {"max": max(b.width, b.height), "min": min(b.width, b.height),
                   "width": b.width, "height": b.height}
            factor = dic[self.unit[5:]]
        return self.size * factor

    def set_size(self, size):
        self.size = size

    def get_center_in_pixels(self):
        return self.ax.transData.transform(self._xydata)

    def set_center(self, xy):
        self._xydata = xy

    def get_theta(self, vec):
        vec_in_pixels = self.ax.transData.transform(vec) - self._center
        return np.rad2deg(np.arctan2(vec_in_pixels[1], vec_in_pixels[0]))

    def get_theta1(self):
        return self.get_theta(self.vec1)

    def get_theta2(self):
        return self.get_theta(self.vec2)

    def set_theta(self, angle):
        pass

    _center = property(get_center_in_pixels, set_center)
    theta1 = property(get_theta1, set_theta)
    theta2 = property(get_theta2, set_theta)
    width = property(get_size, set_size)
    height = property(get_size, set_size)

    def draw(self, renderer):
        self.update_text()
        super().draw(renderer)

    def update_text(self):
        c = self._center
        s = self.get_size()
        angle_span = (self.theta2 - self.theta1) % 360
        angle = np.deg2rad(self.theta1 + angle_span / 2)
        r = s / 2
        if self.textposition == "inside":
            r = s / np.interp(angle_span, [60, 90, 135, 180], [3.3, 3.5, 3.8, 4])
        self.text.xy = c + r * np.array([np.cos(angle), np.sin(angle)])
        if self.textposition == "outside":
            def R90(a, r, w, h):
                if a < np.arctan(h/2/(r+w/2)):
                    return np.sqrt((r+w/2)**2 + (np.tan(a)*(r+w/2))**2)
                else:
                    c = np.sqrt((w/2)**2+(h/2)**2)
                    T = np.arcsin(c * np.cos(np.pi/2 - a + np.arcsin(h/2/c))/r)
                    xy = r * np.array([np.cos(a + T), np.sin(a + T)])
                    xy += np.array([w/2, h/2])
                    return np.sqrt(np.sum(xy**2))

            def R(a, r, w, h):
                aa = (a % (np.pi/4))*((a % (np.pi/2)) <= np.pi/4) + \
                     (np.pi/4 - (a % (np.pi/4)))*((a % (np.pi/2)) >= np.pi/4)
                return R90(aa, r, *[w, h][::int(np.sign(np.cos(2*a)))])

            bbox = self.text.get_window_extent()
            X = R(angle, r, bbox.width, bbox.height)
            trans = self.ax.figure.dpi_scale_trans.inverted()
            offs = trans.transform(((X-s/2), 0))[0] * 72
            self.text.set_position([offs*np.cos(angle), offs*np.sin(angle)])


def _plot_angle(ax, pos, angle, length=0.95, acol="C0", **kwargs):
    vec2 = np.array([np.cos(np.deg2rad(angle)), np.sin(np.deg2rad(angle))])
    xy = np.c_[[length, 0], [0, 0], vec2*length].T + np.array(pos)
    ax.plot(*xy.T, color=acol)
    return AngleAnnotation(pos, xy[0], xy[2], ax=ax, **kwargs)


def _label_angles(ax, texts):
    for x, text in zip([2.0, 3.5, 5.0, 6.5], texts):
        ax.annotate(text, xy=(x, 0), xycoords=ax.get_xaxis_transform(),
                    bbox=dict(boxstyle="round", fc="w"), ha="left", fontsize=8,
                    annotation_clip=True)


@case(f"{G}/angle_annotation_1")
def _(fig):
    ax = fig.subplots()
    fig.canvas.draw()
    ax.set_title("AngleLabel example")
    center = (4.5, 650)
    p1 = [(2.5, 710), (6.0, 605)]
    p2 = [(3.0, 275), (5.5, 900)]
    ax.plot(*zip(*p1))
    ax.plot(*zip(*p2))
    ax.plot(*center, marker="o")
    AngleAnnotation(center, p1[1], p2[1], ax=ax, size=75, text=r"$\alpha$")
    AngleAnnotation(center, p2[1], p1[0], ax=ax, size=35, text=r"$\beta$")
    AngleAnnotation(center, p1[0], p2[0], ax=ax, size=75, text=r"$\gamma$")
    AngleAnnotation(center, p2[0], p1[1], ax=ax, size=35, text=r"$\theta$")
    p = [(6.0, 400), (5.3, 410), (5.6, 300)]
    ax.plot(*zip(*p))
    AngleAnnotation(p[1], p[0], p[2], ax=ax, size=40, text=r"$\Phi$", linestyle="--",
                    color="gray", textposition="outside",
                    text_kw=dict(fontsize=16, color="gray"))


@case(f"{G}/angle_annotation_2")
def _(fig):
    ax1, ax2 = fig.subplots(nrows=2, sharex=True)
    fig.suptitle("AngleLabel keyword arguments")
    fig.canvas.draw()
    ax1.margins(y=0.4)
    ax1.set_title("textposition")
    kw = dict(size=75, unit="points", text=r"$60°$")
    _plot_angle(ax1, (2.0, 0), 60, textposition="inside", **kw)
    _plot_angle(ax1, (3.5, 0), 60, textposition="outside", **kw)
    _plot_angle(ax1, (5.0, 0), 60, textposition="edge",
                text_kw=dict(bbox=dict(boxstyle="round", fc="w")), **kw)
    _plot_angle(ax1, (6.5, 0), 60, textposition="edge",
                text_kw=dict(xytext=(30, 20), arrowprops=dict(arrowstyle="->",
                             connectionstyle="arc3,rad=-0.2")), **kw)
    _label_angles(ax1, ['"inside"', '"outside"', '"edge"', '"edge", custom arrow'])
    ax2.margins(y=0.4)
    ax2.set_title("unit")
    kw = dict(text=r"$60°$", textposition="outside")
    _plot_angle(ax2, (2.0, 0), 60, size=50, unit="pixels", **kw)
    _plot_angle(ax2, (3.5, 0), 60, size=50, unit="points", **kw)
    _plot_angle(ax2, (5.0, 0), 60, size=0.25, unit="axes min", **kw)
    _plot_angle(ax2, (6.5, 0), 60, size=0.25, unit="axes max", **kw)
    _label_angles(ax2, ['"pixels"', '"points"', '"axes min"', '"axes max"'])


# --- angles_on_bracket_arrows -------------------------------------------------


def _rotated_vertical_top(origin, line_length, degrees):
    """The end of a vertical line from *origin* rotated by *degrees*."""
    rad = np.deg2rad(-degrees)
    return [origin[0] + line_length * np.sin(rad), origin[1] + line_length * np.cos(rad)]


@case(f"{G}/angles_on_bracket_arrows")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(0, 6), ylim=(-1, 5))
    ax.set_title("Orientation of the bracket arrows relative to angleA and angleB")
    style = ']-['
    for i, angle in enumerate([-40, 0, 60]):
        y = 2*i
        arrow_centers = ((1, y), (5, y))
        vlines = ((1, y + 0.5), (5, y + 0.5))
        anglesAB = (angle, -angle)
        bracketstyle = f"{style}, angleA={anglesAB[0]}, angleB={anglesAB[1]}"
        bracket = FancyArrowPatch(*arrow_centers, arrowstyle=bracketstyle, mutation_scale=42)
        ax.add_patch(bracket)
        ax.text(3, y + 0.05, bracketstyle, ha="center", va="bottom", fontsize=14)
        ax.vlines([line[0] for line in vlines], [y, y], [line[1] for line in vlines],
                  linestyles="--", color="C0")
        patch_tops = [_rotated_vertical_top(center, 0.5, angle)
                      for center, angle in zip(arrow_centers, anglesAB)]
        connection_dirs = (1, -1) if angle > 0 else (-1, 1)
        arrowstyle = "Simple, tail_width=0.5, head_width=4, head_length=8"
        for vline, dir, patch_top, angle in zip(vlines, connection_dirs, patch_tops, anglesAB):
            kw = dict(connectionstyle=f"arc3,rad={dir * 0.5}", arrowstyle=arrowstyle,
                      color="C0")
            ax.add_patch(FancyArrowPatch(vline, patch_top, **kw))
            ax.text(vline[0] - dir * 0.15, y + 0.7, f'{angle}°', ha="center", va="center")


# --- annotation_basic, annotation_polar and annotation_demo -------------------


def _cosine(ax, **line):
    t = np.arange(0.0, 5.0, 0.01)
    ax.plot(t, np.cos(2*np.pi*t), **line)


def _polar_annotation(ax, **line):
    """A spiral on polar axes, one point of it annotated from the lower left
    corner of the figure."""
    r = np.arange(0, 1, 0.001)
    theta = 2 * 2*np.pi * r
    ax.plot(theta, r, **line)
    ind = 800
    thisr, thistheta = r[ind], theta[ind]
    ax.plot([thistheta], [thisr], 'o')
    ax.annotate('a polar annotation', xy=(thistheta, thisr), xytext=(0.05, 0.05),
                textcoords='figure fraction', arrowprops=dict(facecolor='black', shrink=0.05),
                horizontalalignment='left', verticalalignment='bottom')


@case(f"{G}/annotation_basic")
def _(fig):
    ax = fig.subplots()
    _cosine(ax, lw=2)
    ax.annotate('local max', xy=(2, 1), xytext=(3, 1.5),
                arrowprops=dict(facecolor='black', shrink=0.05))
    ax.set_ylim(-2, 2)


@case(f"{G}/annotation_polar")
def _(fig):
    _polar_annotation(fig.add_subplot(projection='polar'), color='#ee8d18', lw=3)


@case(f"{G}/annotation_demo_1", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    _cosine(ax)
    ax.annotate('figure pixels', xy=(10, 10), xycoords='figure pixels')
    ax.annotate('figure points', xy=(107, 110), xycoords='figure points', fontsize=12)
    ax.annotate('figure fraction', xy=(.025, .975), xycoords='figure fraction',
                horizontalalignment='left', verticalalignment='top', fontsize=20)
    ax.annotate('point offset from data', xy=(3, 1), xycoords='data', xytext=(-10, 90),
                textcoords='offset points', arrowprops=dict(facecolor='black', shrink=0.05),
                horizontalalignment='center', verticalalignment='bottom')
    ax.annotate('axes fraction', xy=(2, 1), xycoords='data', xytext=(0.36, 0.68),
                textcoords='axes fraction', arrowprops=dict(facecolor='black', shrink=0.05),
                horizontalalignment='right', verticalalignment='top')
    ax.annotate('pixel offset from axes fraction', xy=(1, 0), xycoords='axes fraction',
                xytext=(-20, 20), textcoords='offset pixels', horizontalalignment='right',
                verticalalignment='bottom')
    ax.set(xlim=(-1, 5), ylim=(-3, 5))


@case(f"{G}/annotation_demo_2", figsize=(3, 3))
def _(fig):
    _polar_annotation(fig.subplots(subplot_kw=dict(projection='polar')))


@case(f"{G}/annotation_demo_3")
def _(fig):
    el = Ellipse((0, 0), 10, 20, facecolor='r', alpha=0.5)
    ax = fig.subplots(subplot_kw=dict(aspect='equal'))
    ax.add_artist(el)
    el.set_clip_box(ax.bbox)
    ax.annotate('the top', xy=(np.pi/2., 10.), xytext=(np.pi/3, 20.), xycoords='polar',
                textcoords='polar', arrowprops=dict(facecolor='black', shrink=0.05),
                horizontalalignment='left', verticalalignment='bottom', clip_on=True)
    ax.set(xlim=[-20, 20], ylim=[-20, 20])


#: ``annotation_demo``'s connection styles: text, xy, xytext, extra
#: annotate keywords and the arrow's keywords.
CONNECTIONS = [
    ('straight', (0, 1), (-50, 30), {}, {}),
    ('arc3,\nrad 0.2', (0.5, -1), (-80, -60), {}, dict(connectionstyle="arc3,rad=.2")),
    ('arc,\nangle 50', (1., 1), (-90, 50), {},
     dict(connectionstyle="arc,angleA=0,armA=50,rad=10")),
    ('arc,\narms', (1.5, -1), (-80, -60), {},
     dict(connectionstyle="arc,angleA=0,armA=40,angleB=-90,armB=30,rad=7")),
    ('angle,\nangle 90', (2., 1), (-70, 30), {},
     dict(connectionstyle="angle,angleA=0,angleB=90,rad=10")),
    ('angle3,\nangle -90', (2.5, -1), (-80, -60), {},
     dict(connectionstyle="angle3,angleA=0,angleB=-90")),
    ('angle,\nround', (3., 1), (-60, 30), dict(bbox=dict(boxstyle="round", fc="0.8")),
     dict(connectionstyle="angle,angleA=0,angleB=90,rad=10")),
    ('angle,\nround4', (3.5, -1), (-70, -80),
     dict(size=20, bbox=dict(boxstyle="round4,pad=.5", fc="0.8")),
     dict(connectionstyle="angle,angleA=0,angleB=-90,rad=10")),
    ('angle,\nshrink', (4., 1), (-60, 30), dict(bbox=dict(boxstyle="round", fc="0.8")),
     dict(shrinkA=0, shrinkB=10, connectionstyle="angle,angleA=0,angleB=90,rad=10")),
]


@case(f"{G}/annotation_demo_4", figsize=(8, 5))
def _(fig):
    ax = fig.subplots()
    _cosine(ax, lw=3)
    for text, xy, xytext, kw, arrow in CONNECTIONS:
        ax.annotate(text, xy=xy, xycoords='data', xytext=xytext, textcoords='offset points',
                    arrowprops=dict(arrowstyle="->", **arrow), **kw)
    ax.annotate('', xy=(4., 1.), xycoords='data', xytext=(4.5, -1), textcoords='data',
                arrowprops=dict(arrowstyle="<->", connectionstyle="bar", ec="k",
                                shrinkA=5, shrinkB=5))
    ax.set(xlim=(-1, 5), ylim=(-4, 3))


@case(f"{G}/annotation_demo_5")
def _(fig):
    ax = fig.subplots()
    el = Ellipse((2, -1), 0.5, 0.5)
    ax.add_patch(el)
    ax.annotate('$->$', xy=(2., -1), xycoords='data', xytext=(-150, -140),
                textcoords='offset points', bbox=dict(boxstyle="round", fc="0.8"),
                arrowprops=dict(arrowstyle="->", patchB=el,
                                connectionstyle="angle,angleA=90,angleB=0,rad=10"))
    ax.annotate('arrow\nfancy', xy=(2., -1), xycoords='data', xytext=(-100, 60),
                textcoords='offset points', size=20,
                arrowprops=dict(arrowstyle="fancy", fc="0.6", ec="none", patchB=el,
                                connectionstyle="angle3,angleA=0,angleB=-90"))
    ax.annotate('arrow\nsimple', xy=(2., -1), xycoords='data', xytext=(100, 60),
                textcoords='offset points', size=20,
                arrowprops=dict(arrowstyle="simple", fc="0.6", ec="none", patchB=el,
                                connectionstyle="arc3,rad=0.3"))
    ax.annotate('wedge', xy=(2., -1), xycoords='data', xytext=(-100, -100),
                textcoords='offset points', size=20,
                arrowprops=dict(arrowstyle="wedge,tail_width=0.7", fc="0.6", ec="none",
                                patchB=el, connectionstyle="arc3,rad=-0.3"))
    ax.annotate('bubble,\ncontours', xy=(2., -1), xycoords='data', xytext=(0, -70),
                textcoords='offset points', size=20,
                bbox=dict(boxstyle="round", fc=(1.0, 0.7, 0.7), ec=(1., .5, .5)),
                arrowprops=dict(arrowstyle="wedge,tail_width=1.", fc=(1.0, 0.7, 0.7),
                                ec=(1., .5, .5), patchA=None, patchB=el, relpos=(0.2, 0.8),
                                connectionstyle="arc3,rad=-0.1"))
    ax.annotate('bubble', xy=(2., -1), xycoords='data', xytext=(55, 0),
                textcoords='offset points', size=20, va="center",
                bbox=dict(boxstyle="round", fc=(1.0, 0.7, 0.7), ec="none"),
                arrowprops=dict(arrowstyle="wedge,tail_width=1.", fc=(1.0, 0.7, 0.7),
                                ec="none", patchA=None, patchB=el, relpos=(0.2, 0.5)))
    ax.set(xlim=(-1, 5), ylim=(-5, 3))


@case(f"{G}/annotation_demo_6")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    bbox_args = dict(boxstyle="round", fc="0.8")
    arrow_args = dict(arrowstyle="->")
    for coords in ["figure fraction", "axes fraction"]:
        for corner, offset, ha, va in [(0, 20, "left", "bottom"), (1, -20, "right", "top")]:
            ax1.annotate(f'{coords} : {corner}, {corner}', xy=(corner, corner),
                         xycoords=coords, xytext=(offset, offset), textcoords='offset points',
                         ha=ha, va=va, bbox=bbox_args, arrowprops=arrow_args)
    an1 = ax1.annotate('Drag me 1', xy=(.5, .7), xycoords='data', ha="center", va="center",
                       bbox=bbox_args)
    an2 = ax1.annotate('Drag me 2', xy=(.5, .5), xycoords=an1, xytext=(.5, .3),
                       textcoords='axes fraction', ha="center", va="center", bbox=bbox_args,
                       arrowprops=dict(patchB=an1.get_bbox_patch(),
                                       connectionstyle="arc3,rad=0.2", **arrow_args))
    ax1.annotate('', xy=(.5, .5), xycoords=an2, xytext=(.5, .5), textcoords=an1,
                 ha="center", va="center", bbox=bbox_args,
                 arrowprops=dict(patchA=an1.get_bbox_patch(), patchB=an2.get_bbox_patch(),
                                 connectionstyle="arc3,rad=0.2", **arrow_args))
    text = ax2.annotate('xy=(0, 1)\nxycoords=("data", "axes fraction")', xy=(0, 1),
                        xycoords=("data", 'axes fraction'), xytext=(0, -20),
                        textcoords='offset points', ha="center", va="top", bbox=bbox_args,
                        arrowprops=arrow_args)
    ax2.annotate('xy=(0.5, 0)\nxycoords=artist', xy=(0.5, 0.), xycoords=text,
                 xytext=(0, -20), textcoords='offset points', ha="center", va="top",
                 bbox=bbox_args, arrowprops=arrow_args)
    ax2.annotate('xy=(0.8, 0.5)\nxycoords=ax1.transData', xy=(0.8, 0.5),
                 xycoords=ax1.transData, xytext=(10, 10),
                 textcoords=OffsetFrom(ax2.bbox, (0, 0), "points"), ha="left", va="bottom",
                 bbox=bbox_args, arrowprops=arrow_args)
    ax2.set(xlim=[-2, 2], ylim=[-2, 2])


# --- arrow_demo ---------------------------------------------------------------


ARROW_DATA = {
    'A': 0.4, 'T': 0.3, 'G': 0.6, 'C': 0.2,
    'AT': 0.4, 'AC': 0.3, 'AG': 0.2,
    'TA': 0.2, 'TC': 0.3, 'TG': 0.4,
    'CT': 0.2, 'CG': 0.3, 'CA': 0.2,
    'GA': 0.1, 'GT': 0.4, 'GC': 0.1,
}


def _make_arrow_graph(ax, data, size, display, max_arrow_width=0.03, arrow_sep=0.02,
                      alpha=0.5, **kwargs):
    """The example's ``make_arrow_graph`` with ``shape='right'`` and
    ``normalize_data=True`` (which rescales *data* in place)."""
    ax.set(xlim=(-0.25, 1.25), ylim=(-0.25, 1.25), xticks=[], yticks=[],
           title=f'flux encoded as arrow {display}')
    max_text_size = size * 12
    min_text_size = size
    label_text_size = size * 4
    bases = 'ATGC'
    coords = {'A': np.array([0, 1]), 'T': np.array([1, 1]), 'G': np.array([0, 0]),
              'C': np.array([1, 0])}
    colors = {'A': 'r', 'T': 'k', 'G': 'g', 'C': 'b'}
    for base in bases:
        fontsize = np.clip(max_text_size * data[base]**(1/2), min_text_size, max_text_size)
        ax.text(*coords[base], f'${base}_3$', color=colors[base], size=fontsize,
                horizontalalignment='center', verticalalignment='center', weight='bold')
    arrow_h_offset = 0.25
    max_arrow_length = 1 - 2 * arrow_h_offset
    max_head_width = 2.5 * max_arrow_width
    max_head_length = 2 * max_arrow_width
    sf = 0.6
    max_val = max((v for k, v in data.items() if len(k) == 2), default=0)
    for k, v in data.items():
        data[k] = v / max_val * sf
    for pair in map(''.join, itertools.permutations(bases, 2)):
        if display == 'length':
            length = max_head_length + data[pair] / sf * (max_arrow_length - max_head_length)
        else:
            length = max_arrow_length
        if display == 'alpha':
            alpha = min(data[pair] / sf, alpha)
        if display == 'width':
            scale = data[pair] / sf
            width = max_arrow_width * scale
            head_width = max_head_width * scale
            head_length = max_head_length * scale
        else:
            width = max_arrow_width
            head_width = max_head_width
            head_length = max_head_length
        fc = colors[pair[0]]
        cp0 = coords[pair[0]]
        cp1 = coords[pair[1]]
        delta = cos, sin = (cp1 - cp0) / np.hypot(*(cp1 - cp0))
        x_pos, y_pos = ((cp0 + cp1) / 2 - delta * length / 2
                        + np.array([-sin, cos]) * arrow_sep)
        ax.arrow(x_pos, y_pos, cos * length, sin * length, fc=fc, ec=fc, alpha=alpha,
                 width=width, head_width=head_width, head_length=head_length, shape='right',
                 length_includes_head=True, **kwargs)
        orig_positions = {
            'base': [3 * max_arrow_width, 3 * max_arrow_width],
            'center': [length / 2, 3 * max_arrow_width],
            'tip': [length - 3 * max_arrow_width, 3 * max_arrow_width],
        }
        where = 'base' if (cp0 != cp1).all() else 'center'
        M = [[cos, -sin], [sin, cos]]
        x, y = np.dot(M, orig_positions[where]) + [x_pos, y_pos]
        label = r'$r_{_{\mathrm{%s}}}$' % (pair,)
        ax.text(x, y, label, size=label_text_size, ha='center', va='center', color=fc)


@case(f"{G}/arrow_demo", figsize=(12, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    data = dict(ARROW_DATA)
    axs = fig.subplot_mosaic([["length", "width", "alpha"]])
    for display, ax in axs.items():
        _make_arrow_graph(ax, data, size=4, display=display, linewidth=0.001)


# --- autowrap -----------------------------------------------------------------


LONG_TEXT = ("This is a really long string that I'd rather have wrapped so that it "
             "doesn't go outside of the figure, but if it's long enough it will go "
             "off the top or bottom!")


@case(f"{G}/autowrap")
def _(fig):
    ax = fig.add_subplot()
    ax.axis((0, 10, 0, 10))
    ax.text(4, 1, LONG_TEXT, ha='left', rotation=15, wrap=True)
    ax.text(6, 5, LONG_TEXT, ha='left', rotation=15, wrap=True)
    ax.text(5, 5, LONG_TEXT, ha='right', rotation=-15, wrap=True)
    ax.text(5, 10, LONG_TEXT, fontsize=18, style='oblique', ha='center', va='top', wrap=True)
    ax.text(3, 4, LONG_TEXT, family='serif', style='italic', ha='right', wrap=True)
    ax.text(-1, 0, LONG_TEXT, ha='left', rotation=-15, wrap=True)


# --- custom_legends -----------------------------------------------------------


def _coolwarm_lines(fig):
    """Ten noisy geometric ramps coloured along ``coolwarm`` by the
    property cycle; returns the axes and the colormap."""
    data = (np.geomspace(1, 10, 100) + rng(19680801).normal(10 * 100).reshape(10, 100)).T
    cmap = mpl.colormaps["coolwarm"]
    mpl.rcParams['axes.prop_cycle'] = cycler(color=cmap(np.linspace(0, 1, 10)))
    ax = fig.subplots()
    ax.plot(data)
    return ax, cmap


@case(f"{G}/custom_legends_1")
def _(fig):
    _coolwarm_lines(fig)


@case(f"{G}/custom_legends_2")
def _(fig):
    ax, cmap = _coolwarm_lines(fig)
    custom_lines = [mlines.Line2D([0], [0], color=cmap(0.), lw=4),
                    mlines.Line2D([0], [0], color=cmap(.5), lw=4),
                    mlines.Line2D([0], [0], color=cmap(1.), lw=4)]
    ax.legend(custom_lines, ['Cold', 'Medium', 'Hot'])


@case(f"{G}/custom_legends_3")
def _(fig):
    legend_elements = [mlines.Line2D([0], [0], color='b', lw=4, label='Line'),
                       mlines.Line2D([0], [0], marker='o', color='w', label='Scatter',
                                     markerfacecolor='g', markersize=15),
                       Patch(facecolor='orange', edgecolor='r', label='Color Patch')]
    ax = fig.subplots()
    ax.legend(handles=legend_elements, loc='center')


# --- demo_annotation_box ------------------------------------------------------


def _photo():
    """A smooth RGB image the size of the gallery's Grace Hopper photo."""
    y, x = np.mgrid[0:600, 0:512]
    red = 0.5 + 0.5 * np.sin(x / 40.0) * np.cos(y / 55.0)
    green = (x + y) / (512 + 600)
    blue = 0.5 + 0.5 * np.cos(np.hypot(x - 256, y - 250) / 30.0)
    return (np.dstack([red, green, blue]) * 255).astype(np.uint8)


@case(f"{G}/demo_annotation_box_1")
def _(fig):
    fig.set_layout_engine("compressed")
    axd = fig.subplot_mosaic([['t1', '.', 't2']])
    xy1 = (.25, .75)
    xy2 = (.75, .25)
    axd['t1'].plot(*xy1, ".r")
    axd['t2'].plot(*xy2, ".r")
    axd['t1'].set(xlim=(0, 1), ylim=(0, 1), aspect='equal')
    axd['t2'].set(xlim=(0, 1), ylim=(0, 1), aspect='equal')
    c = ConnectionPatch(xyA=xy1, xyB=xy2, coordsA=axd['t1'].transData,
                        coordsB=axd['t2'].transData, arrowstyle='->')
    fig.add_artist(c)
    ab1 = AnnotationBbox(TextArea("Test 1"), xy=(.5, .5), xybox=(0, 30), xycoords=c,
                         boxcoords="offset points", arrowprops=dict(arrowstyle="->"),
                         bboxprops=dict(boxstyle="sawtooth"))
    fig.add_artist(ab1)


@case(f"{G}/demo_annotation_box_2")
def _(fig):
    ax = fig.subplots()
    xy = (0.3, 0.55)
    ax.scatter(*xy, s=200, marker='X')
    im = OffsetImage(np.arange(100).reshape((10, 10)), zoom=2, cmap='viridis')
    im.image.axes = ax
    ab = AnnotationBbox(im, xy=xy, xybox=(-50., 50.), xycoords='data',
                        boxcoords="offset points", pad=0.3, arrowprops=dict(arrowstyle="->"))
    ax.add_artist(ab)
    imagebox = OffsetImage(_photo(), zoom=0.2)
    imagebox.image.axes = ax
    ab = AnnotationBbox(imagebox, xy=xy, xybox=(120., -80.), xycoords='data',
                        boxcoords="offset points", pad=0.5,
                        arrowprops=dict(arrowstyle="->",
                                        connectionstyle="angle,angleA=0,angleB=90,rad=3"))
    ax.add_artist(ab)
    ax.set(xlim=(0, 1), ylim=(0, 1))


@case(f"{G}/demo_annotation_box_3")
def _(fig):
    ax = fig.subplots()
    xy = (0.05, 0.5)
    ax.scatter(*xy, s=500, marker='X')
    da = DrawingArea(120, 120)
    da.add_artist(Circle((30, 30), 25, color='C0'))
    da.add_artist(Annulus((65, 65), 50, 5, color='C1'))
    ab = AnnotationBbox(da, xy=xy, xybox=(.55, xy[1]), xycoords='data',
                        boxcoords=("axes fraction", "data"), box_alignment=(0, 0.5),
                        arrowprops=dict(arrowstyle="->"), bboxprops=dict(alpha=0.5))
    ax.add_artist(ab)
    ax.set(xlim=(0, 1), ylim=(0, 1))


# --- demo_text_rotation_mode --------------------------------------------------


def _rotation_mode(fig, mode):
    ha_list = ["left", "center", "right"]
    va_list = ["top", "center", "baseline", "bottom"]
    axs = fig.subplots(len(va_list), len(ha_list), sharex=True, sharey=True,
                       subplot_kw=dict(aspect=1), gridspec_kw=dict(hspace=0, wspace=0))
    for ha, ax in zip(ha_list, axs[-1, :]):
        ax.set_xlabel(ha)
    for va, ax in zip(va_list, axs[:, 0]):
        ax.set_ylabel(va)
    axs[0, 1].set_title(f"rotation_mode='{mode}'", size="large")
    kw = ({} if mode == "default" else
          {"bbox": dict(boxstyle="square,pad=0.", ec="none", fc="C1", alpha=0.3)})
    texts = {}
    for i, va in enumerate(va_list):
        for j, ha in enumerate(ha_list):
            ax = axs[i, j]
            ax.set(xticks=[], yticks=[])
            ax.axvline(0.5, color="skyblue", zorder=0)
            ax.axhline(0.5, color="skyblue", zorder=0)
            ax.plot(0.5, 0.5, color="C0", marker="o", zorder=1)
            texts[ax] = ax.text(0.5, 0.5, "Tpg", size="x-large", rotation=40,
                                horizontalalignment=ha, verticalalignment=va,
                                rotation_mode=mode, **kw)
    if mode == "default":
        fig.canvas.draw()
        for ax, text in texts.items():
            bb = text.get_window_extent().transformed(ax.transData.inverted())
            ax.add_patch(Rectangle((bb.x0, bb.y0), bb.width, bb.height, facecolor="C1",
                                   alpha=0.3, zorder=2))


@case(f"{G}/demo_text_rotation_mode", figsize=(8, 5))
def _(fig):
    subfigs = fig.subfigures(1, 2)
    _rotation_mode(subfigs[0], "default")
    _rotation_mode(subfigs[1], "anchor")


# --- dfrac_demo, unicode_minus ------------------------------------------------


@case(f"{G}/dfrac_demo", figsize=(5.25, 0.75))
def _(fig):
    fig.text(0.5, 0.3, r'\dfrac: $\dfrac{a}{b}$', horizontalalignment='center',
             verticalalignment='center')
    fig.text(0.5, 0.7, r'\frac: $\frac{a}{b}$', horizontalalignment='center',
             verticalalignment='center')


@case(f"{G}/unicode_minus", figsize=(4, 2))
def _(fig):
    fig.text(.15, .6, "Unicode minus:", fontsize=20)
    fig.text(.85, .6, "\N{MINUS SIGN}1", ha='right', fontsize=20)
    fig.text(.15, .3, "ASCII hyphen:", fontsize=20)
    fig.text(.85, .3, "-1", ha='right', fontsize=20)


# --- fancyarrow_demo ----------------------------------------------------------


def _wrapped_signature(stylecls):
    """The style's default parameters, wrapped at every comma, or every
    second one when there are more than three."""
    s = str(inspect.signature(stylecls))[1:-1]
    n = 2 if s.count(',') > 3 else 1
    return re.sub(', ', lambda m, c=itertools.count(1): m.group() if next(c) % n else '\n', s)


@case(f"{G}/fancyarrow_demo", figsize=(8, 9))
def _(fig):
    styles = ArrowStyle.get_styles()
    ncol = 2
    nrow = (len(styles) + 1) // ncol
    gridspec_kw = dict(wspace=0, hspace=0.05, left=0, right=1, bottom=0, top=1)
    axs = fig.subplots(1 + nrow, ncol, gridspec_kw=gridspec_kw)
    for ax in axs.flat:
        ax.set_xlim(-0.1, 4)
        ax.set_axis_off()
    for ax in axs[0, :]:
        ax.text(0, 0.5, "arrowstyle", size="large", color="tab:blue")
        ax.text(1.4, .5, "default parameters", size="large")
    for ax, (stylename, stylecls) in zip(axs[1:, :].T.flat, styles.items()):
        l, = ax.plot(1.25, 0, "o", color="darkgrey")
        ax.annotate(stylename, (1.25, 0), (0, 0), size="large", color="tab:blue", va="center",
                    family="monospace",
                    arrowprops=dict(arrowstyle=stylename, connectionstyle="arc3,rad=0",
                                    color="black", shrinkA=5, shrinkB=5, patchB=l),
                    bbox=dict(boxstyle="square", fc="w", ec="darkgrey"))
        ax.text(1.4, 0, _wrapped_signature(stylecls), verticalalignment="center", color="0.3")


# --- fancytextbox_demo --------------------------------------------------------


@case(f"{G}/fancytextbox_demo")
def _(fig):
    ax = fig.add_subplot()
    ax.text(0.6, 0.7, "eggs", size=50, rotation=30., ha="center", va="center",
            bbox=dict(boxstyle="round", ec=(1., 0.5, 0.5), fc=(1., 0.8, 0.8)))
    ax.text(0.55, 0.6, "spam", size=50, rotation=-25., ha="right", va="top",
            bbox=dict(boxstyle="square", ec=(1., 0.5, 0.5), fc=(1., 0.8, 0.8)))


# --- figlegend_demo -----------------------------------------------------------


@case(f"{G}/figlegend_demo")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    x = np.arange(0.0, 4*np.pi, 0.2)
    axs[0].plot(x, np.sin(x), label='Line 1')
    axs[0].plot(x, np.exp(-x/2), marker='o', label='Line 2')
    axs[1].plot(x, np.sin(x), color='tab:green', label='Line 3')
    axs[1].plot(x, np.exp(-x/4), color='tab:red', marker='^', label='Line 4')
    fig.legend(loc='outside right upper')


# --- font_family_rc, font_file, font_table ------------------------------------


def _print_text(fig, text):
    fig.set_facecolor("#eefade")
    ax = fig.subplots()
    ax.text(0.5, 0.5, text, ha='center', va='center', size=40)
    ax.axis("off")


@case(f"{G}/font_family_rc_1", figsize=(6, 1), rc={"font.family": "sans-serif"})
def _(fig):
    _print_text(fig, "Hello World! 01")


@case(f"{G}/font_family_rc_3", figsize=(6, 1), rc={"font.family": "monospace"})
def _(fig):
    _print_text(fig, "Hello World! 03")


@case(f"{G}/font_file")
def _(fig):
    ax = fig.subplots()
    fpath = Path(mpl.get_data_path(), "fonts/ttf/cmr10.ttf")
    ax.set_title(f'This is a special font: {fpath.name}', font=fpath)
    ax.set_xlabel('This is the default font')


@case(f"{G}/font_table", figsize=(8, 4))
def _(fig):
    path = fm.findfont(fm.FontProperties())
    font = FT2Font(path)
    codes = font.get_charmap().items()
    labelc = [f"{i:X}" for i in range(16)]
    labelr = [f"{i:02X}" for i in range(0, 16*16, 16)]
    chars = [["" for c in range(16)] for r in range(16)]
    for char_code, glyph_index in codes:
        if char_code >= 256:
            continue
        row, col = divmod(char_code, 16)
        chars[row][col] = chr(char_code)
    ax = fig.subplots()
    ax.set_title(os.path.basename(path))
    ax.set_axis_off()
    table = ax.table(cellText=chars, rowLabels=labelr, colLabels=labelc,
                     rowColours=["palegreen"] * 16, colColours=["palegreen"] * 16,
                     cellColours=[[".95" for c in range(16)] for r in range(16)],
                     cellLoc='center', loc='upper left')
    for key, cell in table.get_celld().items():
        row, col = key
        if row > 0 and col > -1:
            cell.set_text_props(font=Path(path))
    fig.tight_layout()


# --- fonts_demo and fonts_demo_kw ---------------------------------------------


ALIGNMENT = {'horizontalalignment': 'center', 'verticalalignment': 'baseline'}
YP = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2]
FAMILIES = ['serif', 'sans-serif', 'cursive', 'fantasy', 'monospace']
STYLES = ['normal', 'italic', 'oblique']
VARIANTS = ['normal', 'small-caps']
WEIGHTS = ['light', 'normal', 'medium', 'semibold', 'bold', 'heavy', 'black']
SIZES = ['xx-small', 'x-small', 'small', 'medium', 'large', 'x-large', 'xx-large']


@case(f"{G}/fonts_demo")
def _(fig):
    heading_font = FontProperties(size='large')
    fig.text(0.1, 0.9, 'family', fontproperties=heading_font, **ALIGNMENT)
    for k, family in enumerate(FAMILIES):
        font = FontProperties(family=[family])
        fig.text(0.1, YP[k], family, fontproperties=font, **ALIGNMENT)
    fig.text(0.3, 0.9, 'style', fontproperties=heading_font, **ALIGNMENT)
    for k, style in enumerate(STYLES):
        font = FontProperties(family='sans-serif', style=style)
        fig.text(0.3, YP[k], style, fontproperties=font, **ALIGNMENT)
    fig.text(0.5, 0.9, 'variant', fontproperties=heading_font, **ALIGNMENT)
    for k, variant in enumerate(VARIANTS):
        font = FontProperties(family='serif', variant=variant)
        fig.text(0.5, YP[k], variant, fontproperties=font, **ALIGNMENT)
    fig.text(0.7, 0.9, 'weight', fontproperties=heading_font, **ALIGNMENT)
    for k, weight in enumerate(WEIGHTS):
        font = FontProperties(weight=weight)
        fig.text(0.7, YP[k], weight, fontproperties=font, **ALIGNMENT)
    fig.text(0.9, 0.9, 'size', fontproperties=heading_font, **ALIGNMENT)
    for k, size in enumerate(SIZES):
        font = FontProperties(size=size)
        fig.text(0.9, YP[k], size, fontproperties=font, **ALIGNMENT)
    for y, size in [(0.1, 'x-small'), (0.2, 'medium'), (0.3, 'x-large')]:
        font = FontProperties(style='italic', weight='bold', size=size)
        fig.text(0.3, y, 'bold italic', fontproperties=font, **ALIGNMENT)


@case(f"{G}/fonts_demo_kw")
def _(fig):
    fig.text(0.1, 0.9, 'family', size='large', **ALIGNMENT)
    for k, family in enumerate(FAMILIES):
        fig.text(0.1, YP[k], family, family=family, **ALIGNMENT)
    fig.text(0.3, 0.9, 'style', **ALIGNMENT)
    for k, style in enumerate(STYLES):
        fig.text(0.3, YP[k], style, family='sans-serif', style=style, **ALIGNMENT)
    fig.text(0.5, 0.9, 'variant', **ALIGNMENT)
    for k, variant in enumerate(VARIANTS):
        fig.text(0.5, YP[k], variant, family='serif', variant=variant, **ALIGNMENT)
    fig.text(0.7, 0.9, 'weight', **ALIGNMENT)
    for k, weight in enumerate(WEIGHTS):
        fig.text(0.7, YP[k], weight, weight=weight, **ALIGNMENT)
    fig.text(0.9, 0.9, 'size', **ALIGNMENT)
    for k, size in enumerate(SIZES):
        fig.text(0.9, YP[k], size, size=size, **ALIGNMENT)
    for y, size in [(0.1, 'x-small'), (0.2, 'medium'), (0.3, 'x-large')]:
        fig.text(0.3, y, 'bold italic', style='italic', weight='bold', size=size, **ALIGNMENT)


# --- label_subplots -----------------------------------------------------------


def _mosaic(fig):
    fig.set_layout_engine("constrained")
    return fig.subplot_mosaic([['a)', 'c)'], ['b)', 'c)'], ['d)', 'd)']])


@case(f"{G}/label_subplots_1")
def _(fig):
    for label, ax in _mosaic(fig).items():
        ax.annotate(label, xy=(0, 1), xycoords='axes fraction', xytext=(+0.5, -0.5),
                    textcoords='offset fontsize', fontsize='medium', verticalalignment='top',
                    fontfamily='serif', bbox=dict(facecolor='0.7', edgecolor='none', pad=3.0))


@case(f"{G}/label_subplots_2")
def _(fig):
    for label, ax in _mosaic(fig).items():
        ax.text(0.0, 1.0, label,
                transform=ax.transAxes + ScaledTranslation(-20/72, +7/72, fig.dpi_scale_trans),
                fontsize='medium', va='bottom', fontfamily='serif')


@case(f"{G}/label_subplots_3")
def _(fig):
    for label, ax in _mosaic(fig).items():
        ax.set_title('Normal Title', fontstyle='italic')
        ax.set_title(label, fontfamily='serif', loc='left', fontsize='medium')


# --- legend and legend_demo ---------------------------------------------------


@case(f"{G}/legend")
def _(fig):
    a = np.arange(0, 3, .02)
    c = np.exp(a)
    d = c[::-1]
    ax = fig.subplots()
    ax.plot(a, c, 'k--', label='Model length')
    ax.plot(a, d, 'k:', label='Data length')
    ax.plot(a, c + d, 'k', label='Total message length')
    legend = ax.legend(loc='upper center', shadow=True, fontsize='x-large')
    legend.get_frame().set_facecolor('C0')


@case(f"{G}/legend_demo_1")
def _(fig):
    t1 = np.arange(0.0, 2.0, 0.1)
    t2 = np.arange(0.0, 2.0, 0.01)
    ax = fig.subplots()
    ax.plot(t2, np.exp(-t2))
    l2, l3 = ax.plot(t2, np.sin(2 * np.pi * t2), '--o', t1, np.log(1 + t1), '.')
    l4, = ax.plot(t2, np.exp(-t2) * np.sin(2 * np.pi * t2), 's-.')
    ax.legend((l2, l4), ('oscillatory', 'damped'), loc='upper right', shadow=True)
    ax.set_xlabel('time')
    ax.set_ylabel('volts')
    ax.set_title('Damped oscillation')


@case(f"{G}/legend_demo_2")
def _(fig):
    x = np.linspace(0, 1)
    ax0, ax1 = fig.subplots(2, 1)
    for n in range(1, 5):
        ax0.plot(x, x**n, label=f"{n=}")
    leg = ax0.legend(loc="upper left", bbox_to_anchor=[0, 1], ncols=2, shadow=True,
                     title="Legend", fancybox=True)
    leg.get_title().set_color("red")
    ax1.plot(x, x**2, label="multi\nline")
    half_pi = np.linspace(0, np.pi / 2)
    ax1.plot(np.sin(half_pi), np.cos(half_pi), label=r"$\frac{1}{2}\pi$")
    ax1.plot(x, 2**(x**2), label="$2^{x^2}$")
    ax1.legend(shadow=True, fancybox=True)


@case(f"{G}/legend_demo_3")
def _(fig):
    fig.set_layout_engine("constrained")
    top_ax, middle_ax, bottom_ax = fig.subplots(3, 1)
    top_ax.bar([0, 1, 2], [0.2, 0.3, 0.1], width=0.4, label="Bar 1", align="center")
    top_ax.bar([0.5, 1.5, 2.5], [0.3, 0.2, 0.2], color="red", width=0.4, label="Bar 2",
               align="center")
    top_ax.legend()
    middle_ax.errorbar([0, 1, 2], [2, 3, 1], xerr=0.4, fmt="s", label="test 1")
    middle_ax.errorbar([0, 1, 2], [3, 2, 4], yerr=0.3, fmt="o", label="test 2")
    middle_ax.errorbar([0, 1, 2], [1, 1, 3], xerr=0.4, yerr=0.3, fmt="^", label="test 3")
    middle_ax.legend()
    bottom_ax.stem([0.3, 1.5, 2.7], [1, 3.6, 2.7], label="stem test")
    bottom_ax.legend()


@case(f"{G}/legend_demo_4")
def _(fig):
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(2, 1)
    p1 = ax1.scatter([1], [5], c='r', marker='s', s=100)
    p2 = ax1.scatter([3], [2], c='b', marker='o', s=100)
    p3, = ax1.plot([1, 5], [4, 4], 'm-d')
    ax1.legend([(p1, p3), p2], ['two keys', 'one key'], scatterpoints=1, numpoints=1,
               handler_map={tuple: HandlerTuple(ndivide=None)})
    x_left = [1, 2, 3]
    y_pos = [1, 3, 2]
    y_neg = [2, 1, 4]
    rneg = ax2.bar(x_left, y_neg, width=0.5, color='w', hatch='///', label='-1')
    rpos = ax2.bar(x_left, y_pos, width=0.5, color='k', label='+1')
    ax2.legend([(rpos, rneg), (rneg, rpos)], ['pad!=0', 'pad=0'],
               handler_map={(rpos, rneg): HandlerTuple(ndivide=None),
                            (rneg, rpos): HandlerTuple(ndivide=None, pad=0.)})


# --- line_with_text -----------------------------------------------------------


class MyLine(mlines.Line2D):
    """The example's line that draws its label at its last point."""

    def __init__(self, *args, **kwargs):
        self.text = mtext.Text(0, 0, '')
        super().__init__(*args, **kwargs)
        self.text.set_text(self.get_label())

    def set_figure(self, figure):
        self.text.set_figure(figure)
        super().set_figure(figure)

    @mlines.Line2D.axes.setter
    def axes(self, new_axes):
        self.text.axes = new_axes
        mlines.Line2D.axes.fset(self, new_axes)

    def set_transform(self, transform):
        texttrans = transform + mtransforms.Affine2D().translate(2, 2)
        self.text.set_transform(texttrans)
        super().set_transform(transform)

    def set_data(self, x, y):
        if len(x):
            self.text.set_position((x[-1], y[-1]))
        super().set_data(x, y)

    def draw(self, renderer):
        super().draw(renderer)
        self.text.draw(renderer)


@case(f"{G}/line_with_text")
def _(fig):
    ax = fig.subplots()
    g = rng(19680801)
    x, y = g.uniform(20), g.uniform(20)
    line = MyLine(x, y, mfc='red', ms=12, label='line label')
    line.text.set_color('red')
    line.text.set_fontsize(16)
    ax.add_line(line)


# --- mathtext -----------------------------------------------------------------


def _text_to_rgba(s, *, dpi, **kwargs):
    fig = Figure(facecolor="none")
    fig.text(0, 0, s, **kwargs)
    with BytesIO() as buf:
        fig.savefig(buf, dpi=dpi, format="png", bbox_inches="tight", pad_inches=0)
        buf.seek(0)
        rgba = np.asarray(Image.open(buf))
    return rgba


@case(f"{G}/mathtext_asarray")
def _(fig):
    rgba1 = _text_to_rgba(r"IQ: $\sigma_i=15$", color="blue", fontsize=20, dpi=200)
    rgba2 = _text_to_rgba(r"some other string", color="red", fontsize=20, dpi=200)
    fig.figimage(rgba1, 100, 50)
    fig.figimage(rgba2, 100, 150)
    fig.text(100, 250, r"IQ: $\sigma_i=15$", color="blue", fontsize=20,
             transform=IdentityTransform())
    fig.text(100, 350, r"some other string", color="red", fontsize=20,
             transform=IdentityTransform())


@case(f"{G}/mathtext_demo")
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3], label=r'$\sqrt{x^2}$')
    ax.legend()
    ax.set_xlabel(r'$\Delta_i^j$', fontsize=20)
    ax.set_ylabel(r'$\Delta_{i+1}^j$', fontsize=20)
    ax.set_title(r'$\Delta_i^j \hspace{0.4} \mathrm{versus} \hspace{0.4} '
                 r'\Delta_{i+1}^j$', fontsize=20)
    tex = r'$\mathcal{R}\prod_{i=\alpha_{i+1}}^\infty a_i\sin(2 \pi f x_i)$'
    ax.text(1, 1.6, tex, fontsize=20, va='bottom')
    fig.tight_layout()


MATHTEXT_DEMOS = {
    "Header demo":
        r"$W^{3\beta}_{\delta_1 \rho_1 \sigma_2} = "
        r"U^{3\beta}_{\delta_1 \rho_1} + \frac{1}{8 \pi 2} "
        r"\int^{\alpha_2}_{\alpha_2} d \alpha^\prime_2 \left[\frac{ "
        r"U^{2\beta}_{\delta_1 \rho_1} - \alpha^\prime_2U^{1\beta}_"
        r"{\rho_1 \sigma_2} }{U^{0\beta}_{\rho_1 \sigma_2}}\right]$",
    "Subscripts and superscripts":
        r"$\alpha_i > \beta_i,\ "
        r"\alpha_{i+1}^j = {\rm sin}(2\pi f_j t_i) e^{-5 t_i/\tau},\ "
        r"\ldots$",
    "Fractions, binomials and stacked numbers":
        r"$\frac{3}{4},\ \binom{3}{4},\ \genfrac{}{}{0}{}{3}{4},\ "
        r"\left(\frac{5 - \frac{1}{x}}{4}\right),\ \ldots$",
    "Radicals":
        r"$\sqrt{2},\ \sqrt[3]{x},\ \ldots$",
    "Fonts":
        r"$\mathrm{Roman}\ , \ \mathit{Italic}\ , \ \mathtt{Typewriter} \ "
        r"\mathrm{or}\ \mathcal{CALLIGRAPHY}$",
    "Accents":
        r"$\acute a,\ \bar a,\ \breve a,\ \dot a,\ \ddot a, \ \grave a, \ "
        r"\hat a,\ \tilde a,\ \vec a,\ \widehat{xyz},\ \widetilde{xyz},\ "
        r"\ldots$",
    "Greek, Hebrew":
        r"$\alpha,\ \beta,\ \chi,\ \delta,\ \lambda,\ \mu,\ "
        r"\Delta,\ \Gamma,\ \Omega,\ \Phi,\ \Pi,\ \Upsilon,\ \nabla,\ "
        r"\aleph,\ \beth,\ \daleth,\ \gimel,\ \ldots$",
    "Delimiters, functions and Symbols":
        r"$\coprod,\ \int,\ \oint,\ \prod,\ \sum,\ "
        r"\log,\ \sin,\ \approx,\ \oplus,\ \star,\ \varpropto,\ "
        r"\infty,\ \partial,\ \Re,\ \leftrightsquigarrow, \ \ldots$",
}


@case(f"{G}/mathtext_examples", figsize=(7, 7))
def _(fig):
    mpl_grey_rgb = (51 / 255, 51 / 255, 51 / 255)
    ax = fig.add_axes((0.01, 0.01, 0.98, 0.90), facecolor="white", frameon=True)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.set_title("Matplotlib's math rendering engine", color=mpl_grey_rgb, fontsize=14,
                 weight='bold')
    ax.set_xticks([])
    ax.set_yticks([])
    line_axesfrac = 1 / len(MATHTEXT_DEMOS)
    ax.annotate(MATHTEXT_DEMOS['Header demo'], xy=(0.5, 1. - 0.59 * line_axesfrac),
                color='tab:orange', ha='center', fontsize=20)
    for i_line, (title, demo) in enumerate(MATHTEXT_DEMOS.items()):
        if i_line == 0:
            continue
        baseline = 1 - i_line * line_axesfrac
        baseline_next = baseline - line_axesfrac
        fill_color = ['white', 'tab:blue'][i_line % 2]
        ax.axhspan(baseline, baseline_next, color=fill_color, alpha=0.2)
        ax.annotate(f'{title}:', xy=(0.06, baseline - 0.3 * line_axesfrac),
                    color=mpl_grey_rgb, weight='bold')
        ax.annotate(demo, xy=(0.04, baseline - 0.75 * line_axesfrac), color=mpl_grey_rgb,
                    fontsize=16)


@case(f"{G}/mathtext_fontfamily_example", figsize=(6, 5))
def _(fig):
    ax = fig.subplots()
    ax.plot(range(11), color="0.9")
    msg = (r"Normal Text. $Text\ in\ math\ mode:\ "
           r"\int_{0}^{\infty } x^2 dx$")
    ax.text(1, 7, msg, size=12, math_fontfamily='cm')
    ax.text(1, 3, msg, size=12, math_fontfamily='dejavuserif')
    ax.set_title(r"$Title\ in\ math\ mode:\ \int_{0}^{\infty } x^2 dx$",
                 math_fontfamily='stixsans', size=14)


@case(f"{G}/stix_fonts_demo", figsize=(8, 11))
def _(fig):
    circle123 = "\N{CIRCLED DIGIT ONE}\N{CIRCLED DIGIT TWO}\N{CIRCLED DIGIT THREE}"
    tests = [
        r'$%s\;\mathrm{%s}\;\mathbf{%s}$' % ((circle123,) * 3),
        r'$\mathsf{Sans \Omega}\;\mathrm{\mathsf{Sans \Omega}}\;'
        r'\mathbf{\mathsf{Sans \Omega}}$',
        r'$\mathtt{Monospace}$',
        r'$\mathcal{CALLIGRAPHIC}$',
        r'$\mathbb{Blackboard\;\pi}$',
        r'$\mathrm{\mathbb{Blackboard\;\pi}}$',
        r'$\mathbf{\mathbb{Blackboard\;\pi}}$',
        r'$\mathfrak{Fraktur}\;\mathbf{\mathfrak{Fraktur}}$',
        r'$\mathscr{Script}$',
    ]
    for i, s in enumerate(tests[::-1]):
        fig.text(0, (i + .5) / len(tests), s, fontsize=32)


# --- multiline ----------------------------------------------------------------


@case(f"{G}/multiline_1")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.bar(range(6), [0.3, 0.5, 0.8, 1.5, 2.5, 8], alpha=0.3)
    ax.set_ylabel('labels are centered,\nso is their multi-line text by default')
    ax.set_xlabel('xlabel with\nmultialignment="left"', multialignment="left")
    ax.text(4.5, 6, 'these lines\nare right aligned\nbecause of horizontalalignment',
            horizontalalignment='right')
    ax.text(4.5, 4, 'but\nwe can override this\nwith an explicit multialingment="left"',
            horizontalalignment='right', multialignment='left')


@case(f"{G}/multiline_2")
def _(fig):
    ax = fig.subplots()
    for x, text in [(0.1, "Mat\nTTp\n123"), (0.3, "Mag\nTTT\n123"),
                    (0.5, "Mag\nTTT$^{A^A}$\n123")]:
        ax.text(x, 0.4, text, size=18, va="baseline", bbox=dict(fc="none", ec="tab:orange"))
    ax.axhline(0.4)
    ax.set_title("test line spacing for multiline text")
    ax.set_xlim(0, 0.7)
    ax.yaxis.minorticks_on()
    ax.yaxis.set_tick_params(which="minor", grid_color="0.9")
    ax.grid(which="both")


# --- placing_text_boxes, rainbow_text, watermark_text -------------------------


@case(f"{G}/placing_text_boxes")
def _(fig):
    ax = fig.subplots()
    x = 30 * rng(19680801).normal(10000)
    mu = x.mean()
    median = np.median(x)
    sigma = x.std()
    textstr = '\n'.join((
        r'$\mu=%.2f$' % (mu, ),
        r'$\mathrm{median}=%.2f$' % (median, ),
        r'$\sigma=%.2f$' % (sigma, )))
    ax.hist(x, 50)
    props = dict(boxstyle='round', facecolor='wheat', alpha=0.5)
    ax.text(0.05, 0.95, textstr, transform=ax.transAxes, fontsize=14,
            verticalalignment='top', bbox=props)


@case(f"{G}/rainbow_text", rc={"font.size": 20})
def _(fig):
    ax = fig.add_subplot(xticks=[], yticks=[])
    text = ax.text(.1, .5, "Matplotlib", color="red")
    text = ax.annotate(" says,", xycoords=text, xy=(1, 0), verticalalignment="bottom",
                       color="gold", weight="bold")
    text = ax.annotate(" hello", xycoords=text, xy=(1, 0), verticalalignment="bottom",
                       color="green", style="italic")
    ax.annotate(" world!", xycoords=text, xy=(1, 0), verticalalignment="bottom",
                color="blue", family="serif")


@case(f"{G}/watermark_text")
def _(fig):
    ax = fig.subplots()
    ax.plot(rng(19680801).uniform(20), '-o', ms=20, lw=2, alpha=0.7, mfc='orange')
    ax.grid()
    ax.text(0.5, 0.5, 'created with matplotlib', transform=ax.transAxes, fontsize=40,
            color='gray', alpha=0.5, ha='center', va='center', rotation=30)


# --- text_alignment -----------------------------------------------------------


@case(f"{G}/text_alignment_1", figsize=(6, 4))
def _(fig):
    # The figure of the example's docstring.
    y = [0.22, 0.34, 0.5, 0.56, 0.78]
    x = [0.17, 0.5, 0.855]
    X, Y = np.meshgrid(x, y)
    ax = fig.subplots()
    ax.set(xlim=(0, 1), ylim=(0, 1), xticks=[], yticks=[])
    ax.spines[:].set_visible(False)
    ax.text(0.5, 0.5, 'plot', fontsize=128, ha='center', va='center', zorder=1)
    ax.hlines(y, x[0], x[-1], color='grey')
    ax.vlines(x, y[0], y[-1], color='grey')
    ax.plot(X.ravel(), Y.ravel(), 'o')
    pad_x = 0.02
    pad_y = 0.04
    for y_i, label in zip(y, ['bottom', 'baseline', 'center', 'center_baseline', 'top']):
        ax.text(x[0] - pad_x, y_i, label, ha='right', va='center')
    for x_i, label in zip(x, ['left', 'center', 'right']):
        ax.text(x_i, y[0] - pad_y, label, ha='center', va='top')
    ax.set_xlabel('horizontalalignment', fontsize=14)
    ax.set_ylabel('verticalalignment', fontsize=14, labelpad=35)
    ax.set_title('Relative position of text anchor point depending on alignment')


@case(f"{G}/text_alignment_2")
def _(fig):
    ax = fig.subplots()
    left, width = .25, .5
    bottom, height = .25, .5
    right = left + width
    top = bottom + height
    p = Rectangle((left, bottom), width, height, fill=False)
    p.set_transform(ax.transAxes)
    p.set_clip_on(False)
    ax.add_patch(p)
    middle = 0.5 * (bottom + top)
    for x, y, text, ha, va, rotation in [
        (left, bottom, 'left top', 'left', 'top', 0),
        (left, bottom, 'left bottom', 'left', 'bottom', 0),
        (right, top, 'right bottom', 'right', 'bottom', 0),
        (right, top, 'right top', 'right', 'top', 0),
        (right, bottom, 'center top', 'center', 'top', 0),
        (left, middle, 'right center', 'right', 'center', 'vertical'),
        (left, middle, 'left center', 'left', 'center', 'vertical'),
        (0.5 * (left + right), middle, 'middle', 'center', 'center', 0),
        (right, middle, 'centered', 'center', 'center', 'vertical'),
        (left, top, 'rotated\nwith newlines', 'center', 'center', 45),
    ]:
        ax.text(x, y, text, horizontalalignment=ha, verticalalignment=va, rotation=rotation,
                transform=ax.transAxes)
    ax.set_axis_off()


# --- text_commands, text_fontdict, text_rotation_relative_to_line -------------


@case(f"{G}/text_commands")
def _(fig):
    fig.suptitle('bold figure suptitle', fontsize=14, fontweight='bold')
    ax = fig.add_subplot()
    fig.subplots_adjust(top=0.85)
    ax.set_title('axes title')
    ax.set_xlabel('xlabel')
    ax.set_ylabel('ylabel')
    ax.text(3, 8, 'boxed italics text in data coords', style='italic',
            bbox={'facecolor': 'red', 'alpha': 0.5, 'pad': 10})
    ax.text(2, 6, r'an equation: $E=mc^2$', fontsize=15)
    ax.text(3, 2, 'Unicode: Institut f\374r Festk\366rperphysik')
    ax.text(0.95, 0.01, 'colored text in axes coords', verticalalignment='bottom',
            horizontalalignment='right', transform=ax.transAxes, color='green', fontsize=15)
    ax.plot([2], [1], 'o')
    ax.annotate('annotate', xy=(2, 1), xytext=(3, 4),
                arrowprops=dict(facecolor='black', shrink=0.05))
    ax.set(xlim=(0, 10), ylim=(0, 10))


@case(f"{G}/text_fontdict")
def _(fig):
    font = {'family': 'serif', 'color': 'darkred', 'weight': 'normal', 'size': 16}
    x = np.linspace(0.0, 5.0, 100)
    y = np.cos(2*np.pi*x) * np.exp(-x)
    ax = fig.add_subplot()
    ax.plot(x, y, 'k')
    ax.set_title('Damped exponential decay', fontdict=font)
    ax.text(2, 0.65, r'$\cos(2 \pi t) \exp(-t)$', fontdict=font)
    ax.set_xlabel('time (s)', fontdict=font)
    ax.set_ylabel('voltage (mV)', fontdict=font)
    fig.subplots_adjust(left=0.15)


@case(f"{G}/text_rotation_relative_to_line")
def _(fig):
    ax = fig.subplots()
    ax.plot(range(0, 8), range(0, 8))
    ax.set_xlim([-10, 10])
    ax.text(-8, 0, 'text 45° in screen coordinates', fontsize=18, rotation=45,
            rotation_mode='anchor')
    ax.text(0, 0, 'text 45° in data coordinates', fontsize=18, rotation=45,
            rotation_mode='anchor', transform_rotates_text=True)


# --- titles_demo --------------------------------------------------------------


@case(f"{G}/titles_demo_1")
def _(fig):
    ax = fig.add_subplot()
    ax.plot(range(10))
    ax.set_title('Center Title')
    ax.set_title('Left Title', loc='left')
    ax.set_title('Right Title', loc='right')


@case(f"{G}/titles_demo_2")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    for ax in axs:
        ax.plot(range(10))
        ax.xaxis.set_label_position('top')
        ax.set_xlabel('X-label')
        ax.set_title('Center Title')
    axs[1].xaxis.tick_top()


@case(f"{G}/titles_demo_3")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    ax = axs[0]
    ax.plot(range(10))
    ax.xaxis.set_label_position('top')
    ax.set_xlabel('X-label')
    ax.set_title('Manual y', y=1.0, pad=-14)
    mpl.rcParams['axes.titley'] = 1.0
    mpl.rcParams['axes.titlepad'] = -14
    ax = axs[1]
    ax.plot(range(10))
    ax.set_xlabel('X-label')
    ax.set_title('rcParam y')
