"""
The frames of the side-by-side clip, composed as the clips of the user's
own comparison project: a running clock and a caption at the top, a
progress bar with a mark at each library's time, and the two pictures
popping in at their measured times, matplotlib's on the left and pltrs'
on the right, ending on which library had the figure first and by how
much.

The pictures are each library's own rendering of the figure, each on a
plain card whose title strip names the library, its version and its time.
"""

from dataclasses import dataclass

from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageOps

#: The frame's size and rate.
W, H = 1920, 1080
FPS = 30
#: Seconds after the later card, and of a card's pop.
HOLD = 2.6
POP = 0.22
#: The height of a card's picture before it is fitted to the frame.
PICTURE_HEIGHT = 560
#: A card's title strip, its padding around the picture, its corners.
STRIP = 48
PAD = 12
RADIUS = 14
#: The room a card's shadow takes on each side, and the gap of the cards.
SHADOW = 36
GAP = 28
#: Where the cards' tops are.
TOP = 168
#: The colours of the drawn background (top, bottom) and of the cards.
SKY = ((17, 24, 39), (49, 64, 92))
INK = (15, 23, 42, 255)
MUTED = (100, 116, 139, 255)
STRIP_FILL = (241, 245, 249, 255)
RULE = (226, 232, 240, 255)


@dataclass(frozen=True)
class Fonts:
    """The clip's four type sizes."""

    clock: ImageFont.ImageFont
    caption: ImageFont.ImageFont
    title: ImageFont.ImageFont
    detail: ImageFont.ImageFont


def load_fonts(regular=None, bold=None):
    """The clip's fonts from the files *regular* and *bold*; Pillow's own
    font where a file is None."""
    def font(path, size):
        return ImageFont.truetype(path, size) if path else ImageFont.load_default(size)

    return Fonts(font(bold, 42), font(regular, 20), font(bold, 19), font(regular, 17))


@dataclass(frozen=True)
class Side:
    """One library's part of the clip."""

    library: str
    version: str
    seconds: float
    picture: Image.Image


@dataclass
class Clip:
    """What every frame of one clip shares."""

    scene: Image.Image
    #: matplotlib's side, then pltrs'.
    sides: tuple
    #: The cards, in the order of `sides`.
    cards: tuple
    description: str
    fonts: Fonts

    @property
    def duration(self):
        """The clip's length in seconds."""
        return max(side.seconds for side in self.sides) + POP + HOLD


def background(path=None):
    """The scene behind the cards: the image at *path* filling the frame,
    or a quiet gradient."""
    if path is not None:
        with Image.open(path) as image:
            return ImageOps.fit(image.convert("RGB"), (W, H), Image.Resampling.LANCZOS).convert("RGBA")
    top, bottom = (Image.new("RGB", (W, H), colour) for colour in SKY)
    return Image.composite(bottom, top, Image.linear_gradient("L").resize((W, H))).convert("RGBA")


def make_clip(scene, sides, description, fonts):
    """The `Clip` of *sides* (matplotlib's, then pltrs') on *scene*."""
    width, height = _picture_box([side.picture for side in sides])
    cards = tuple(_with_shadow(_card(side, width, height, fonts)) for side in sides)
    return Clip(scene, tuple(sides), cards, description, fonts)


def _picture_box(pictures):
    """One size for both cards' pictures: as wide as the wider picture
    needs at `PICTURE_HEIGHT`, smaller if two cards would not fit."""
    aspect = max(picture.width / picture.height for picture in pictures)
    width = round(PICTURE_HEIGHT * aspect)
    room = (W - 40 - GAP) // 2 - 2 * SHADOW - 2 * PAD
    if width > room:
        width = room
    return width, round(width / aspect)


def _card(side, width, height, fonts):
    """*side*'s card: a title strip over its picture, fitted into a
    *width* × *height* box on white."""
    size = (width + 2 * PAD, STRIP + height + 2 * PAD)
    card = Image.new("RGBA", size, (255, 255, 255, 255))
    draw = ImageDraw.Draw(card)
    draw.rectangle((0, 0, size[0], STRIP), fill=STRIP_FILL)
    draw.line((0, STRIP, size[0], STRIP), fill=RULE, width=1)
    middle = STRIP // 2
    draw.text((PAD + 4, middle), side.library, font=fonts.title, fill=INK, anchor="lm")
    name_width = draw.textlength(side.library + "  ", font=fonts.title)
    draw.text((PAD + 4 + name_width, middle), side.version, font=fonts.detail, fill=MUTED, anchor="lm")
    draw.text((size[0] - PAD - 4, middle), f"{side.seconds:.2f} s", font=fonts.title, fill=INK, anchor="rm")
    picture = ImageOps.contain(side.picture.convert("RGBA"), (width, height), Image.Resampling.LANCZOS)
    card.alpha_composite(picture, (PAD + (width - picture.width) // 2, STRIP + PAD + (height - picture.height) // 2))
    mask = Image.new("L", size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size[0] - 1, size[1] - 1), radius=RADIUS, fill=255)
    card.putalpha(mask)
    return card


def _with_shadow(card):
    """*card* on a transparent canvas with a soft shadow under it."""
    canvas = Image.new("RGBA", (card.width + SHADOW * 2, card.height + SHADOW * 2), (0, 0, 0, 0))
    shade = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    ImageDraw.Draw(shade).rounded_rectangle(
        (SHADOW - 2, SHADOW + 10, SHADOW + card.width + 2, SHADOW + card.height + 14),
        radius=RADIUS + 4, fill=(0, 0, 0, 110))
    canvas.alpha_composite(shade.filter(ImageFilter.GaussianBlur(16)))
    canvas.alpha_composite(card, (SHADOW, SHADOW))
    return canvas


def ease(u):
    """The cubic ease-out of a card's pop, *u* from 0 to 1."""
    u = max(0.0, min(1.0, u))
    return 1 - (1 - u) ** 3


def _paste_card(frame, card, cx, opened_for):
    """Pastes *card* centred on *cx*, *opened_for* seconds after it
    appeared: growing and fading in during `POP`, absent before."""
    if opened_for < 0:
        return
    u = ease(opened_for / POP)
    if u < 1:
        scale = 0.94 + 0.06 * u
        shown = card.resize((round(card.width * scale), round(card.height * scale)), Image.Resampling.LANCZOS)
        shown.putalpha(shown.getchannel("A").point(lambda value: int(value * u)))
    else:
        shown = card
    frame.alpha_composite(shown, (cx - shown.width // 2, TOP + (card.height - shown.height) // 2))


def caption_at(clip, t):
    """The caption under the clock at *t*: the script's description until
    the later card is in, then which library had the figure first."""
    first, second = sorted(clip.sides, key=lambda side: side.seconds)
    if t < second.seconds + POP:
        return clip.description
    return f"{first.library} had the figure {second.seconds - first.seconds:.2f} s earlier"


def _header(clip, t):
    """The clock, the caption and the progress bar at *t*, on a
    transparent overlay."""
    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    fonts = clip.fonts
    clock = f"{t:0.2f} s"
    caption = caption_at(clip, t)
    pill_w = max(draw.textlength(clock, font=fonts.clock), draw.textlength(caption, font=fonts.caption)) + 56
    draw.rounded_rectangle(((W - pill_w) // 2, 28, (W + pill_w) // 2, 118), radius=18, fill=(12, 24, 36, 150))
    draw.text((W // 2, 46), clock, font=fonts.clock, fill=(255, 255, 255, 235), anchor="mt")
    draw.text((W // 2, 92), caption, font=fonts.caption, fill=(255, 255, 255, 210), anchor="mt")
    bar_w = 520
    bar = ((W - bar_w) // 2, 132, (W + bar_w) // 2, 142)
    draw.rounded_rectangle(bar, radius=5, fill=(255, 255, 255, 70))
    filled = bar_w * min(1.0, t / clip.duration)
    if filled > 2:
        draw.rounded_rectangle((bar[0], bar[1], bar[0] + filled, bar[3]), radius=5, fill=(255, 255, 255, 230))
    for side in clip.sides:
        mark = bar[0] + bar_w * (side.seconds / clip.duration)
        draw.rectangle((mark - 1, bar[1] - 4, mark + 1, bar[3] + 4), fill=(255, 255, 255, 230))
    return overlay


def card_centres(clip):
    """The horizontal centres of the two cards in the frame."""
    left_card, right_card = clip.cards
    left = (W - (left_card.width + GAP + right_card.width)) // 2
    return left + left_card.width // 2, left + left_card.width + GAP + right_card.width // 2


def frame_at(clip, t):
    """The clip's frame *t* seconds after the start, as an RGB image."""
    frame = clip.scene.copy()
    frame.alpha_composite(_header(clip, t))
    for side, card, centre in zip(clip.sides, clip.cards, card_centres(clip)):
        _paste_card(frame, card, centre, t - side.seconds)
    return frame.convert("RGB")


def frames(clip, fps=FPS):
    """Every frame of *clip* at *fps* frames a second."""
    for index in range(round(clip.duration * fps)):
        yield frame_at(clip, index / fps)
