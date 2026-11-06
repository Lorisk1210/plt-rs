"""
matplotlib 3.11.2's docstrings of the box, connection and arrow styles
(``patches.BoxStyle.Square``, ...), under the Matplotlib License: pltrs
makes the style classes in `._patch_styles` (`document`).
"""

#: (container, style) -> (the docstring of the class, that of its
#: constructor, the nearest a base gives where it has none of its own).
DOCS = {
    ('BoxStyle', 'Square'): (
        'A square box.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
        ),
    ),
    ('BoxStyle', 'Circle'): (
        'A circular box.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
        ),
    ),
    ('BoxStyle', 'Ellipse'): (
        (
            '\n'
            'An elliptical box.\n'
            '\n'
            '.. versionadded:: 3.7\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
        ),
    ),
    ('BoxStyle', 'RArrow'): (
        'A box in the shape of a right-pointing arrow.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'head_width : float, default: 1.5\n'
            '    The head width, relative to the arrow shaft width; must be\n'
            '    nonnegative.\n'
            'head_angle : float, default: 90\n'
            '    The angle at the tip of the arrow, in degrees; must be nonzero\n'
            '    (modulo 360).  Negative angles result in arrow heads pointing\n'
            '    backwards.\n'
        ),
    ),
    ('BoxStyle', 'LArrow'): (
        'A box in the shape of a left-pointing arrow.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'head_width : float, default: 1.5\n'
            '    The head width, relative to the arrow shaft width; must be\n'
            '    nonnegative.\n'
            'head_angle : float, default: 90\n'
            '    The angle at the tip of the arrow, in degrees; must be nonzero\n'
            '    (modulo 360).  Negative angles result in arrow heads pointing\n'
            '    backwards.\n'
        ),
    ),
    ('BoxStyle', 'DArrow'): (
        'A box in the shape of a two-way arrow.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'head_width : float, default: 1.5\n'
            '    The head width, relative to the arrow shaft width; must be\n'
            '    nonnegative.\n'
            'head_angle : float, default: 90\n'
            '    The angle at the tip of the arrow, in degrees; must be nonzero\n'
            '    (modulo 360).  Negative angles result in arrow heads pointing\n'
            '    backwards.\n'
        ),
    ),
    ('BoxStyle', 'Round'): (
        'A box with round corners.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'rounding_size : float, default: *pad*\n'
            '    Radius of the corners.\n'
        ),
    ),
    ('BoxStyle', 'Round4'): (
        'A box with rounded edges.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'rounding_size : float, default: *pad*/2\n'
            '    Rounding of edges.\n'
        ),
    ),
    ('BoxStyle', 'Sawtooth'): (
        'A box with a sawtooth outline.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'tooth_size : float, default: *pad*/2\n'
            '    Size of the sawtooth.\n'
        ),
    ),
    ('BoxStyle', 'Roundtooth'): (
        'A box with a rounded sawtooth outline.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'pad : float, default: 0.3\n'
            '    The amount of padding around the original box.\n'
            'tooth_size : float, default: *pad*/2\n'
            '    Size of the sawtooth.\n'
        ),
    ),
    ('ConnectionStyle', 'Arc3'): (
        (
            '\n'
            'Creates a simple quadratic Bézier curve between two\n'
            'points. The curve is created so that the middle control point\n'
            '(C1) is located at the same distance from the start (C0) and\n'
            'end points(C2) and the distance of the C1 to the line\n'
            'connecting C0-C2 is *rad* times the distance of C0-C2.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'rad : float\n'
            '  Curvature of the curve.\n'
        ),
    ),
    ('ConnectionStyle', 'Angle3'): (
        (
            '\n'
            'Creates a simple quadratic Bézier curve between two points. The middle\n'
            'control point is placed at the intersecting point of two lines which\n'
            'cross the start and end point, and have a slope of *angleA* and\n'
            '*angleB*, respectively.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'angleA : float\n'
            '  Starting angle of the path.\n'
            '\n'
            'angleB : float\n'
            '  Ending angle of the path.\n'
        ),
    ),
    ('ConnectionStyle', 'Angle'): (
        (
            '\n'
            'Creates a piecewise continuous quadratic Bézier path between two\n'
            'points. The path has a one passing-through point placed at the\n'
            'intersecting point of two lines which cross the start and end point,\n'
            'and have a slope of *angleA* and *angleB*, respectively.\n'
            'The connecting edges are rounded with *rad*.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'angleA : float\n'
            '  Starting angle of the path.\n'
            '\n'
            'angleB : float\n'
            '  Ending angle of the path.\n'
            '\n'
            'rad : float\n'
            '  Rounding radius of the edge.\n'
        ),
    ),
    ('ConnectionStyle', 'Arc'): (
        (
            '\n'
            'Creates a piecewise continuous quadratic Bézier path between two\n'
            'points. The path can have two passing-through points, a\n'
            'point placed at the distance of *armA* and angle of *angleA* from\n'
            'point A, another point with respect to point B. The edges are\n'
            'rounded with *rad*.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'angleA : float\n'
            '  Starting angle of the path.\n'
            '\n'
            'angleB : float\n'
            '  Ending angle of the path.\n'
            '\n'
            'armA : float or None\n'
            '  Length of the starting arm.\n'
            '\n'
            'armB : float or None\n'
            '  Length of the ending arm.\n'
            '\n'
            'rad : float\n'
            '  Rounding radius of the edges.\n'
        ),
    ),
    ('ConnectionStyle', 'Bar'): (
        (
            '\n'
            'A line with *angle* between A and B with *armA* and *armB*. One of the\n'
            'arms is extended so that they are connected in a right angle. The\n'
            'length of *armA* is determined by (*armA* + *fraction* x AB distance).\n'
            'Same for *armB*.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'armA : float\n'
            '    Minimum length of armA.\n'
            '\n'
            'armB : float\n'
            '    Minimum length of armB.\n'
            '\n'
            'fraction : float\n'
            '    A fraction of the distance between two points that will be\n'
            '    added to armA and armB.\n'
            '\n'
            'angle : float or None\n'
            '    Angle of the connecting line (if None, parallel to A and B).\n'
        ),
    ),
    ('ArrowStyle', 'Curve'): (
        'A simple curve without any arrow head.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'CurveA'): (
        'An arrow with a head at its start point.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'CurveB'): (
        'An arrow with a head at its end point.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'CurveAB'): (
        'An arrow with heads both at the start and the end point.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'CurveFilledA'): (
        'An arrow with filled triangle head at the start.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'CurveFilledB'): (
        'An arrow with filled triangle head at the end.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'CurveFilledAB'): (
        'An arrow with filled triangle heads at both ends.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head, relative to *mutation_size*.\n'
            'head_width : float, default: 0.2\n'
            '    Width of the arrow head, relative to *mutation_size*.\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
            'scaleA, scaleB : float, default: *mutation_size*\n'
            '    The scale of the brackets.\n'
        ),
    ),
    ('ArrowStyle', 'BracketA'): (
        'An arrow with an outward square bracket at its start.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'widthA : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA : float, default: 0 degrees\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
        ),
    ),
    ('ArrowStyle', 'BracketB'): (
        'An arrow with an outward square bracket at its end.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleB : float, default: 0 degrees\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
        ),
    ),
    ('ArrowStyle', 'BracketAB'): (
        'An arrow with outward square brackets at both ends.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA, lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA, angleB : float, default: 0 degrees\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
        ),
    ),
    ('ArrowStyle', 'BarAB'): (
        'An arrow with vertical bars ``|`` at both ends.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'widthA, widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'angleA, angleB : float, default: 0 degrees\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
        ),
    ),
    ('ArrowStyle', 'BracketCurve'): (
        (
            '\n'
            'An arrow with an outward square bracket at its start and a head at\n'
            'the end.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'widthA : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthA : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleA : float, default: 0 degrees\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
        ),
    ),
    ('ArrowStyle', 'CurveBracket'): (
        (
            '\n'
            'An arrow with an outward square bracket at its end and a head at\n'
            'the start.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'widthB : float, default: 1.0\n'
            '    Width of the bracket.\n'
            'lengthB : float, default: 0.2\n'
            '    Length of the bracket.\n'
            'angleB : float, default: 0 degrees\n'
            '    Orientation of the bracket, as a counterclockwise angle.\n'
            '    0 degrees means perpendicular to the line.\n'
        ),
    ),
    ('ArrowStyle', 'Simple'): (
        'A simple arrow. Only works with a quadratic Bézier curve.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.5\n'
            '    Length of the arrow head.\n'
            '\n'
            'head_width : float, default: 0.5\n'
            '    Width of the arrow head.\n'
            '\n'
            'tail_width : float, default: 0.2\n'
            '    Width of the arrow tail.\n'
        ),
    ),
    ('ArrowStyle', 'Fancy'): (
        'A fancy arrow. Only works with a quadratic Bézier curve.',
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'head_length : float, default: 0.4\n'
            '    Length of the arrow head.\n'
            '\n'
            'head_width : float, default: 0.4\n'
            '    Width of the arrow head.\n'
            '\n'
            'tail_width : float, default: 0.4\n'
            '    Width of the arrow tail.\n'
        ),
    ),
    ('ArrowStyle', 'Wedge'): (
        (
            '\n'
            'Wedge(?) shape. Only works with a quadratic Bézier curve.  The\n'
            'start point has a width of the *tail_width* and the end point has a\n'
            'width of 0. At the middle, the width is *shrink_factor*x*tail_width*.\n'
        ),
        (
            '\n'
            'Parameters\n'
            '----------\n'
            'tail_width : float, default: 0.3\n'
            '    Width of the tail.\n'
            '\n'
            'shrink_factor : float, default: 0.5\n'
            '    Fraction of the arrow width at the middle point.\n'
        ),
    ),
}


def document(style, container):
    """Give the style class *style* of *container* matplotlib's docstrings."""
    style.__doc__, style.__init__.__doc__ = DOCS[container.__name__, style.__name__]
