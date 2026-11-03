"""
The texts of axis artists made on their own (matplotlib's `LabelBase`,
`AxisLabel` and `TickLabels` made by their constructors and added with
``ax.add_artist``): their properties, kept in Python, and their
description for pltrs (Rust), which draws them
(``_native.axisartist.add_axis_texts``) and is handed it again whenever a
property changes once an axes added them.
"""

from pltrs import rcParams
from pltrs.font_manager import FontManager, FontProperties
from pltrs._native import axisartist as _axisartist


class OwnTexts:
    """
    The properties of texts made on their own, of *kind* ``"label"``
    (`LabelBase`: *offset_radius* pixels from the point), ``"axislabel"``
    (`AxisLabel`: its pad away) or ``"ticklabels"`` (`TickLabels`: one
    text per location, angle and label of *locs_angles_labels*), with
    matplotlib's `.Text` defaults (rotated around their anchor, as
    `LabelBase` sets).
    """

    def __init__(self, kind, x=0, y=0, text=''):
        self.kind = kind
        self.props = {
            "x": x, "y": y, "text": str(text), "rotation": 0.0,
            "ha": "left", "va": "baseline", "color": rcParams["text.color"],
            "size": rcParams["font.size"],
            "look": (FontManager._props(FontProperties()), None, None, "anchor"),
            "pad": 5, "axis_direction": "bottom", "ref_angle": 0,
            "offset_radius": 0.0, "locs_angles_labels": [],
        }

    def set_axis_direction(self, d):
        """Take the alignment and angle of the conventions of axis
        direction *d* (``set_axis_direction``)."""
        va, ha, angle = _axisartist.axis_direction_defaults(self.kind, d)
        self.props.update(va=va, ha=ha, rotation=angle, axis_direction=d)

    def description(self):
        """The description of the texts `add_axis_texts` takes."""
        props = self.props
        if self.kind == "ticklabels":
            texts = [((float(x), float(y)), float(a), str(label))
                     for (x, y), a, label in props["locs_angles_labels"]]
        else:
            texts = [((float(props["x"]), float(props["y"])),
                      float(props["ref_angle"]), props["text"])]
        return (self.kind, float(props["offset_radius"]), texts,
                (float(props["pad"]), props["color"], float(props["size"]),
                 props["look"], float(props["rotation"]),
                 (props["va"], props["ha"]), props["axis_direction"]))
