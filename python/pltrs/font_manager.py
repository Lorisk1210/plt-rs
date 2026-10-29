"""
A module for finding, managing, and using fonts across platforms
(matplotlib's ``font_manager``).

This module provides a single `FontManager` instance, ``fontManager``, that
can be shared across backends and platforms.  The `findfont`
function returns the best TrueType (TTF) font file in the local or
system font path that matches the specified `FontProperties`
instance.

plt-rs finds the fonts and matches them (``plt::fonts``): it knows
matplotlib's bundled fonts, the fonts of the system and those added with
`FontManager.addfont`, and ranks them as matplotlib does. The property
tables, the system font discovery, `FontProperties` and the JSON
serialization are matplotlib's (from matplotlib 3.11.2, under the
Matplotlib License).
"""

from __future__ import annotations

from base64 import b64encode
import dataclasses
from functools import cache, lru_cache
import functools
import json
import logging
from numbers import Integral
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys

from . import _api, cbook
from ._fontconfig_pattern import (
    parse_fontconfig_pattern, generate_fontconfig_pattern)
from ._native import font_manager as _font_manager

_log = logging.getLogger(__name__)

#: The values of :rc:`mathtext.fontset`.
_MATH_FONTSETS = ('dejavusans', 'dejavuserif', 'cm', 'stix', 'stixsans', 'custom')


def _rcParams():
    """The current rcParams (the package's, imported once it exists)."""
    from . import rcParams
    return rcParams


def _val_or_rc(val, *rc_names):
    """*val*, or else the first rcParam of *rc_names* that is not None."""
    if val is not None:
        return val
    for name in rc_names:
        if (val := _rcParams()[name]) is not None:
            return val
    return val


font_scalings = {
    'xx-small': 0.579,
    'x-small':  0.694,
    'small':    0.833,
    'medium':   1.0,
    'large':    1.200,
    'x-large':  1.440,
    'xx-large': 1.728,
    'larger':   1.2,
    'smaller':  0.833,
    None:       1.0,
}
stretch_dict = {
    'ultra-condensed': 100,
    'extra-condensed': 200,
    'condensed':       300,
    'semi-condensed':  400,
    'normal':          500,
    'semi-expanded':   600,
    'semi-extended':   600,
    'expanded':        700,
    'extended':        700,
    'extra-expanded':  800,
    'extra-extended':  800,
    'ultra-expanded':  900,
    'ultra-extended':  900,
}
weight_dict = {
    'ultralight': 100,
    'light':      200,
    'normal':     400,
    'regular':    400,
    'book':       400,
    'medium':     500,
    'roman':      500,
    'semibold':   600,
    'demibold':   600,
    'demi':       600,
    'bold':       700,
    'heavy':      800,
    'extra bold': 800,
    'black':      900,
}
_weight_regexes = [
    # From fontconfig's FcFreeTypeQueryFaceInternal; not the same as
    # weight_dict!
    ("thin", 100),
    ("extralight", 200),
    ("ultralight", 200),
    ("demilight", 350),
    ("semilight", 350),
    ("light", 300),  # Needs to come *after* demi/semilight!
    ("book", 380),
    ("regular", 400),
    ("normal", 400),
    ("medium", 500),
    ("demibold", 600),
    ("demi", 600),
    ("semibold", 600),
    ("extrabold", 800),
    ("superbold", 800),
    ("ultrabold", 800),
    ("bold", 700),  # Needs to come *after* extra/super/ultrabold!
    ("ultrablack", 1000),
    ("superblack", 1000),
    ("extrablack", 1000),
    (r"\bultra", 1000),
    ("black", 900),  # Needs to come *after* ultra/super/extrablack!
    ("heavy", 900),
]
font_family_aliases = {
    'serif',
    'sans-serif',
    'sans serif',
    'cursive',
    'fantasy',
    'monospace',
    'sans',
}

# OS Font paths
try:
    _HOME = Path.home()
except Exception:  # Exceptions thrown by home() are not specified...
    _HOME = Path(os.devnull)  # Just an arbitrary path with no children.
MSFolders = \
    r'Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders'
MSFontDirectories = [
    r'SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts',
    r'SOFTWARE\Microsoft\Windows\CurrentVersion\Fonts']
MSUserFontDirectories = [
    str(_HOME / 'AppData/Local/Microsoft/Windows/Fonts'),
    str(_HOME / 'AppData/Roaming/Microsoft/Windows/Fonts'),
]
X11FontDirectories = [
    # an old standard installation point
    "/usr/X11R6/lib/X11/fonts/TTF/",
    "/usr/X11/lib/X11/fonts",
    # here is the new standard location for fonts
    "/usr/share/fonts/",
    # documented as a good place to install new fonts
    "/usr/local/share/fonts/",
    # common application, not really useful
    "/usr/lib/openoffice/share/fonts/truetype/",
    # user fonts
    str((Path(os.environ.get('XDG_DATA_HOME') or _HOME / ".local/share"))
        / "fonts"),
    str(_HOME / ".fonts"),
]
OSXFontDirectories = [
    "/Library/Fonts/",
    "/Network/Library/Fonts/",
    "/System/Library/Fonts/",
    # fonts installed via MacPorts
    "/opt/local/share/fonts",
    # user fonts
    str(_HOME / "Library/Fonts"),
]


def _normalize_weight(weight):
    return weight if isinstance(weight, Integral) else weight_dict[weight]


def get_fontext_synonyms(fontext):
    """
    Return a list of file extensions that are synonyms for
    the given file extension *fileext*.
    """
    return {
        'afm': ['afm'],
        'otf': ['otf', 'ttc', 'ttf'],
        'ttc': ['otf', 'ttc', 'ttf'],
        'ttf': ['otf', 'ttc', 'ttf'],
    }[fontext]


def list_fonts(directory, extensions):
    """
    Return a list of all fonts matching any of the extensions, found
    recursively under the directory.
    """
    extensions = ["." + ext for ext in extensions]
    return [os.path.join(dirpath, filename)
            # os.walk ignores access errors, unlike Path.glob.
            for dirpath, _, filenames in os.walk(directory)
            for filename in filenames
            if Path(filename).suffix.lower() in extensions]


def win32FontDirectory():
    r"""
    Return the user-specified font directory for Win32.  This is
    looked up from the registry key ::

      \\HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders\Fonts

    If the key is not found, ``%WINDIR%\Fonts`` will be returned.
    """  # noqa: E501
    import winreg
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, MSFolders) as user:
            return winreg.QueryValueEx(user, 'Fonts')[0]
    except OSError:
        return os.path.join(os.environ['WINDIR'], 'Fonts')


def _get_win32_installed_fonts():
    """List the font paths known to the Windows registry."""
    import winreg
    items = set()
    # Search and resolve fonts listed in the registry.
    for domain, base_dirs in [
            (winreg.HKEY_LOCAL_MACHINE, [win32FontDirectory()]),  # System.
            (winreg.HKEY_CURRENT_USER, MSUserFontDirectories),  # User.
    ]:
        for base_dir in base_dirs:
            for reg_path in MSFontDirectories:
                try:
                    with winreg.OpenKey(domain, reg_path) as local:
                        for j in range(winreg.QueryInfoKey(local)[1]):
                            # value may contain the filename of the font or its
                            # absolute path.
                            key, value, tp = winreg.EnumValue(local, j)
                            if not isinstance(value, str):
                                continue
                            try:
                                # If value contains already an absolute path,
                                # then it is not changed further.
                                path = Path(base_dir, value).resolve()
                            except RuntimeError:
                                # Don't fail with invalid entries.
                                continue
                            items.add(path)
                except (OSError, MemoryError):
                    continue
    return items


@cache
def _get_fontconfig_fonts():
    """Cache and list the font paths known to ``fc-list``."""
    try:
        if b'--format' not in subprocess.check_output(['fc-list', '--help']):
            _log.warning(  # fontconfig 2.7 implemented --format.
                'Matplotlib needs fontconfig>=2.7 to query system fonts.')
            return []
        out = subprocess.check_output(['fc-list', '--format=%{file}\\n'])
    except (OSError, subprocess.CalledProcessError):
        return []
    return [Path(os.fsdecode(fname)) for fname in out.split(b'\n')]


@cache
def _get_macos_fonts():
    """Cache and list the font paths known to ``system_profiler SPFontsDataType``."""
    try:
        d, = plistlib.loads(
            subprocess.check_output(["system_profiler", "-xml", "SPFontsDataType"]))
    except (OSError, subprocess.CalledProcessError, plistlib.InvalidFileException):
        return []
    return [Path(entry["path"]) for entry in d["_items"]]


def findSystemFonts(fontpaths=None, fontext='ttf'):
    """
    Find fonts in a search path, system paths, or some other platform-specific method.

    Parameters
    ----------
    fontpaths : list of str, optional
        Search for fonts in these specified font paths. If no paths are given and the
        :envvar:`MPL_IGNORE_SYSTEM_FONTS` is not set, use a standard set of system
        paths, as well as the list of fonts tracked by fontconfig if fontconfig is
        installed and available.
    fontext : {'ttf', 'afm'}, default: 'ttf'
        If 'ttf', search for TrueType fonts; if 'afm', search for with AFM fonts.

    Returns
    -------
    list of str
        A list of file paths with fonts of the given type.
    """
    fontfiles = set()
    fontexts = get_fontext_synonyms(fontext)

    if fontpaths is None:
        if os.getenv('MPL_IGNORE_SYSTEM_FONTS'):
            installed_fonts = []
            fontpaths = []
        elif sys.platform == 'win32':
            installed_fonts = _get_win32_installed_fonts()
            fontpaths = []
        elif sys.platform == 'emscripten':
            installed_fonts = []
            fontpaths = []
        else:
            installed_fonts = _get_fontconfig_fonts()
            if sys.platform == 'darwin':
                installed_fonts += _get_macos_fonts()
                fontpaths = [*X11FontDirectories, *OSXFontDirectories]
            else:
                fontpaths = X11FontDirectories
        fontfiles.update(str(path) for path in installed_fonts
                         if path.suffix.lower()[1:] in fontexts)

    elif isinstance(fontpaths, str):
        fontpaths = [fontpaths]

    for path in fontpaths:
        fontfiles.update(map(os.path.abspath, list_fonts(path, fontexts)))

    return [fname for fname in fontfiles if os.path.exists(fname)]


# To maintain backwards-compatibility with the current code we need to continue to
# return a str. However to support indexing into the file we need to return both the
# path and the index. Thus, we sub-class str to maintain compatibility and extend it to
# carry the index.
#
# The other alternative would be to create a completely new API and deprecate the
# existing one. In this case, sub-classing str is the simpler and less-disruptive
# option.
class FontPath(str):
    """
    A class to describe a path to a font with a face index.

    Parameters
    ----------
    path : str
        The path to a font.
    face_index : int
        The face index in the font.
    """

    __match_args__ = ('path', 'face_index')

    def __new__(cls, path, face_index):
        ret = super().__new__(cls, path)
        ret._face_index = face_index
        return ret

    @property
    def path(self):
        """The path to a font."""
        return str(self)

    @property
    def face_index(self):
        """The face index in a font."""
        return self._face_index

    def _as_tuple(self):
        return (self.path, self.face_index)

    def __eq__(self, other):
        if isinstance(other, FontPath):
            return self._as_tuple() == other._as_tuple()
        return super().__eq__(other)

    def __ne__(self, other):
        return not (self == other)

    def __lt__(self, other):
        if isinstance(other, FontPath):
            return self._as_tuple() < other._as_tuple()
        return super().__lt__(other)

    def __gt__(self, other):
        return not (self == other or self < other)

    def __hash__(self):
        return hash(self._as_tuple())

    def __repr__(self):
        return f'FontPath{self._as_tuple()}'


@dataclasses.dataclass(frozen=True)
class FontEntry:
    """
    A class for storing Font properties.

    It is used when populating the font lookup dictionary.
    """

    fname: str = ''
    index: int = 0
    name: str = ''
    style: str = 'normal'
    variant: str = 'normal'
    weight: str | int = 'normal'
    stretch: str = 'normal'
    size: str = 'medium'

    def _repr_html_(self) -> str:
        png_stream = self._repr_png_()
        png_b64 = b64encode(png_stream).decode()
        return f"<img src=\"data:image/png;base64, {png_b64}\" />"

    def _repr_png_(self) -> bytes:
        from matplotlib.figure import Figure  # Circular import.
        fig = Figure()
        font_path = Path(self.fname) if self.fname != '' else None
        fig.text(0, 0, self.name, font=font_path)
        with BytesIO() as buf:
            fig.savefig(buf, bbox_inches='tight', transparent=True)
            return buf.getvalue()


def _cleanup_fontproperties_init(init_method):
    """
    A decorator to limit the call signature to a single positional argument
    or alternatively only keyword arguments.

    We still accept but deprecate all other call signatures.

    When the deprecation expires we can switch the signature to::

        __init__(self, pattern=None, /, *, family=None, style=None, ...)

    plus a runtime check that pattern is not used alongside with the
    keyword arguments. This results eventually in the two possible
    call signatures::

        FontProperties(pattern)
        FontProperties(family=..., size=..., ...)

    """
    @functools.wraps(init_method)
    def wrapper(self, *args, **kwargs):
        # multiple args with at least some positional ones
        if len(args) > 1 or len(args) == 1 and kwargs:
            # Note: Both cases were previously handled as individual properties.
            # Therefore, we do not mention the case of font properties here.
            _api.warn_deprecated(
                "3.10",
                message="Passing individual properties to FontProperties() "
                        "positionally was deprecated in Matplotlib %(since)s and "
                        "will be removed in %(removal)s. Please pass all properties "
                        "via keyword arguments."
            )
        # single non-string arg -> clearly a family not a pattern
        if len(args) == 1 and not kwargs and not cbook.is_scalar_or_string(args[0]):
            # Case font-family list passed as single argument
            _api.warn_deprecated(
                "3.10",
                message="Passing family as positional argument to FontProperties() "
                        "was deprecated in Matplotlib %(since)s and will be removed "
                        "in %(removal)s. Please pass family names as keyword"
                        "argument."
            )
        # Note on single string arg:
        # This has been interpreted as pattern so far. We are already raising if a
        # non-pattern compatible family string was given. Therefore, we do not need
        # to warn for this case.
        return init_method(self, *args, **kwargs)

    return wrapper


class FontProperties:
    """
    A class for storing and manipulating font properties.

    The font properties are the six properties described in the
    `W3C Cascading Style Sheet, Level 1
    <http://www.w3.org/TR/1998/REC-CSS2-19980512/>`_ font
    specification and *math_fontfamily* for math fonts:

    - family: A list of font names in decreasing order of priority.
      The items may include a generic font family name, either 'sans-serif',
      'serif', 'cursive', 'fantasy', or 'monospace'.  In that case, the actual
      font to be used will be looked up from the associated rcParam during the
      search process in `.findfont`. Default: :rc:`font.family`

    - style: Either 'normal', 'italic' or 'oblique'.
      Default: :rc:`font.style`

    - variant: Either 'normal' or 'small-caps'.
      Default: :rc:`font.variant`

    - stretch: A numeric value in the range 0-1000 or one of
      'ultra-condensed', 'extra-condensed', 'condensed',
      'semi-condensed', 'normal', 'semi-expanded', 'expanded',
      'extra-expanded' or 'ultra-expanded'. Default: :rc:`font.stretch`

    - weight: A numeric value in the range 0-1000 or one of
      'ultralight', 'light', 'normal', 'regular', 'book', 'medium',
      'roman', 'semibold', 'demibold', 'demi', 'bold', 'heavy',
      'extra bold', 'black'. Default: :rc:`font.weight`

    - size: Either a relative value of 'xx-small', 'x-small',
      'small', 'medium', 'large', 'x-large', 'xx-large' or an
      absolute font size, e.g., 10. Default: :rc:`font.size`

    - math_fontfamily: The family of fonts used to render math text.
      Supported values are: 'dejavusans', 'dejavuserif', 'cm',
      'stix', 'stixsans' and 'custom'. Default: :rc:`mathtext.fontset`

    Alternatively, a font may be specified using the absolute path to a font
    file, by using the *fname* kwarg.  However, in this case, it is typically
    simpler to just pass the path (as a `pathlib.Path`, not a `str`) to the
    *font* kwarg of the `.Text` object.

    The preferred usage of font sizes is to use the relative values,
    e.g.,  'large', instead of absolute font sizes, e.g., 12.  This
    approach allows all text sizes to be made larger or smaller based
    on the font manager's default font size.

    This class accepts a single positional string as fontconfig_ pattern_,
    or alternatively individual properties as keyword arguments::

        FontProperties(pattern)
        FontProperties(*, family=None, style=None, variant=None, ...)

    This support does not depend on fontconfig; we are merely borrowing its
    pattern syntax for use here.

    .. _fontconfig: https://www.freedesktop.org/wiki/Software/fontconfig/
    .. _pattern:
       https://www.freedesktop.org/software/fontconfig/fontconfig-user.html

    Note that Matplotlib's internal font manager and fontconfig use a
    different algorithm to lookup fonts, so the results of the same pattern
    may be different in Matplotlib than in other applications that use
    fontconfig.
    """

    @_cleanup_fontproperties_init
    def __init__(self, family=None, style=None, variant=None, weight=None,
                 stretch=None, size=None,
                 fname=None,  # if set, it's a hardcoded filename to use
                 math_fontfamily=None):
        self.set_family(family)
        self.set_style(style)
        self.set_variant(variant)
        self.set_weight(weight)
        self.set_stretch(stretch)
        self.set_file(fname)
        self.set_size(size)
        self.set_math_fontfamily(math_fontfamily)
        # Treat family as a fontconfig pattern if it is the only parameter
        # provided.  Even in that case, call the other setters first to set
        # attributes not specified by the pattern to the rcParams defaults.
        if (isinstance(family, str)
                and style is None and variant is None and weight is None
                and stretch is None and size is None and fname is None):
            self.set_fontconfig_pattern(family)

    @classmethod
    def _from_any(cls, arg):
        """
        Generic constructor which can build a `.FontProperties` from any of the
        following:

        - a `.FontProperties`: it is passed through as is;
        - `None`: a `.FontProperties` using rc values is used;
        - an `os.PathLike`: it is used as path to the font file;
        - a `str`: it is parsed as a fontconfig pattern;
        - a `dict`: it is passed as ``**kwargs`` to `.FontProperties`.
        """
        if arg is None:
            return cls()
        elif isinstance(arg, cls):
            return arg
        elif isinstance(arg, os.PathLike):
            return cls(fname=arg)
        elif isinstance(arg, str):
            return cls(arg)
        else:
            return cls(**arg)

    def __hash__(self):
        return hash(tuple(self.__dict__.values()))

    def __eq__(self, other):
        return hash(self) == hash(other)

    def __str__(self):
        return self.get_fontconfig_pattern()

    def get_family(self):
        """
        Return a list of individual font family names or generic family names.

        The font families or generic font families (which will be resolved
        from their respective rcParams when searching for a matching font) in
        the order of preference.
        """
        return list(self._family)

    def get_name(self):
        """
        Return the name of the font that best matches the font properties.
        """
        return fontManager._family_name(findfont(self))

    def get_style(self):
        """
        Return the font style.  Values are: 'normal', 'italic' or 'oblique'.
        """
        return self._slant

    def get_variant(self):
        """
        Return the font variant.  Values are: 'normal' or 'small-caps'.
        """
        return self._variant

    def get_weight(self):
        """
        Get the font weight.  Options are: A numeric value in the
        range 0-1000 or one of 'light', 'normal', 'regular', 'book',
        'medium', 'roman', 'semibold', 'demibold', 'demi', 'bold',
        'heavy', 'extra bold', 'black'
        """
        return self._weight

    def get_stretch(self):
        """
        Return the font stretch or width.  Options are: 'ultra-condensed',
        'extra-condensed', 'condensed', 'semi-condensed', 'normal',
        'semi-expanded', 'expanded', 'extra-expanded', 'ultra-expanded'.
        """
        return self._stretch

    def get_size(self):
        """
        Return the font size.
        """
        return self._size

    def get_file(self):
        """
        Return the filename of the associated font.
        """
        return self._file

    def get_fontconfig_pattern(self):
        """
        Get a fontconfig_ pattern_ suitable for looking up the font as
        specified with fontconfig's ``fc-match`` utility.

        This support does not depend on fontconfig; we are merely borrowing its
        pattern syntax for use here.
        """
        return generate_fontconfig_pattern(self)

    def set_family(self, family):
        """
        Change the font family.  Can be either an alias (generic name
        is CSS parlance), such as: 'serif', 'sans-serif', 'cursive',
        'fantasy', or 'monospace', a real font name or a list of real
        font names.  Real font names are not supported when
        :rc:`text.usetex` is `True`. Default: :rc:`font.family`
        """
        family = _val_or_rc(family, 'font.family')
        if isinstance(family, str):
            family = (family,)
        self._family = tuple(family)

    def set_style(self, style):
        """
        Set the font style.

        Parameters
        ----------
        style : {'normal', 'italic', 'oblique'}, default: :rc:`font.style`
        """
        style = _val_or_rc(style, 'font.style')
        _api.check_in_list(['normal', 'italic', 'oblique'], style=style)
        self._slant = style

    def set_variant(self, variant):
        """
        Set the font variant.

        Parameters
        ----------
        variant : {'normal', 'small-caps'}, default: :rc:`font.variant`
        """
        variant = _val_or_rc(variant, 'font.variant')
        _api.check_in_list(['normal', 'small-caps'], variant=variant)
        self._variant = variant

    def set_weight(self, weight):
        """
        Set the font weight.

        Parameters
        ----------
        weight : int or {'ultralight', 'light', 'normal', 'regular', 'book', \
'medium', 'roman', 'semibold', 'demibold', 'demi', 'bold', 'heavy', \
'extra bold', 'black'}, default: :rc:`font.weight`
            If int, must be in the range  0-1000.
        """
        weight = _val_or_rc(weight, 'font.weight')
        if weight in weight_dict:
            self._weight = weight
            return
        try:
            weight = int(weight)
        except ValueError:
            pass
        else:
            if 0 <= weight <= 1000:
                self._weight = weight
                return
        raise ValueError(f"{weight=} is invalid")

    def set_stretch(self, stretch):
        """
        Set the font stretch or width.

        Parameters
        ----------
        stretch : int or {'ultra-condensed', 'extra-condensed', 'condensed', \
'semi-condensed', 'normal', 'semi-expanded', 'expanded', 'extra-expanded', \
'ultra-expanded'}, default: :rc:`font.stretch`
            If int, must be in the range  0-1000.
        """
        stretch = _val_or_rc(stretch, 'font.stretch')
        if stretch in stretch_dict:
            self._stretch = stretch
            return
        try:
            stretch = int(stretch)
        except ValueError:
            pass
        else:
            if 0 <= stretch <= 1000:
                self._stretch = stretch
                return
        raise ValueError(f"{stretch=} is invalid")

    def set_size(self, size):
        """
        Set the font size.

        Parameters
        ----------
        size : float or {'xx-small', 'x-small', 'small', 'medium', \
'large', 'x-large', 'xx-large'}, default: :rc:`font.size`
            If a float, the font size in points. The string values denote
            sizes relative to the default font size.
        """
        size = _val_or_rc(size, 'font.size')
        try:
            size = float(size)
        except ValueError:
            try:
                scale = font_scalings[size]
            except KeyError as err:
                raise ValueError(
                    "Size is invalid. Valid font size are "
                    + ", ".join(map(str, font_scalings))) from err
            else:
                size = scale * FontManager.get_default_size()
        if size < 1.0:
            _log.info('Fontsize %1.2f < 1.0 pt not allowed by FreeType. '
                      'Setting fontsize = 1 pt', size)
            size = 1.0
        self._size = size

    def set_file(self, file):
        """
        Set the filename of the fontfile to use.  In this case, all
        other properties will be ignored.
        """
        self._file = os.fspath(file) if file is not None else None

    def set_fontconfig_pattern(self, pattern):
        """
        Set the properties by parsing a fontconfig_ *pattern*.

        This support does not depend on fontconfig; we are merely borrowing its
        pattern syntax for use here.
        """
        for key, val in parse_fontconfig_pattern(pattern).items():
            if type(val) is list:
                getattr(self, "set_" + key)(val[0])
            else:
                getattr(self, "set_" + key)(val)

    def get_math_fontfamily(self):
        """
        Return the name of the font family used for math text.

        The default font is :rc:`mathtext.fontset`.
        """
        return self._math_fontfamily

    def set_math_fontfamily(self, fontfamily):
        """
        Set the font family for text in math mode.

        If not set explicitly, :rc:`mathtext.fontset` will be used.

        Parameters
        ----------
        fontfamily : str
            The name of the font family.

            Available font families are defined in the
            :ref:`default matplotlibrc file <customizing-with-matplotlibrc-files>`.

        See Also
        --------
        .text.Text.get_math_fontfamily
        """
        if fontfamily is None:
            fontfamily = _rcParams()['mathtext.fontset']
        else:
            valid_fonts = _MATH_FONTSETS
            # _check_in_list() Validates the parameter math_fontfamily as
            # if it were passed to rcParams['mathtext.fontset']
            _api.check_in_list(valid_fonts, math_fontfamily=fontfamily)
        self._math_fontfamily = fontfamily

    def __copy__(self):
        # Bypass __init__ for speed, since values are already validated
        new = FontProperties.__new__(FontProperties)
        new.__dict__.update(self.__dict__)
        return new

    def copy(self):
        """Return a copy of self."""
        return self.__copy__()

    # Aliases
    set_name = set_family
    get_slant = get_style
    set_slant = set_style
    get_size_in_points = get_size


class _JSONEncoder(json.JSONEncoder):
    def default(self, o):
        if isinstance(o, FontManager):
            return dict(o.__dict__, __class__='FontManager')
        elif isinstance(o, FontEntry):
            d = dict(o.__dict__, __class__='FontEntry')
            try:
                # Cache paths of fonts shipped with Matplotlib relative to the
                # Matplotlib data path, which helps in the presence of venvs.
                d["fname"] = str(Path(d["fname"]))
            except ValueError:
                pass
            return d
        else:
            return super().default(o)


def _json_decode(o):
    cls = o.pop('__class__', None)
    if cls is None:
        return o
    elif cls == 'FontManager':
        r = FontManager.__new__(FontManager)
        r.__dict__.update(o)
        return r
    elif cls == 'FontEntry':
        if not os.path.isabs(o['fname']):
            o['fname'] = o['fname']
        r = FontEntry(**o)
        return r
    else:
        raise ValueError("Don't know how to deserialize __class__=%s" % cls)


def json_dump(data, filename):
    """
    Dump `FontManager` *data* as JSON to the file named *filename*.

    See Also
    --------
    json_load

    Notes
    -----
    File paths that are children of the Matplotlib data path (typically, fonts
    shipped with Matplotlib) are stored relative to that data path (to remain
    valid across virtualenvs).

    This function temporarily locks the output file to prevent multiple
    processes from overwriting one another's output.
    """
    try:
        with cbook._lock_path(filename), open(filename, 'w') as fh:
            json.dump(data, fh, cls=_JSONEncoder, indent=2)
    except OSError as e:
        _log.warning('Could not save font_manager cache %s', e)


def json_load(filename):
    """
    Load a `FontManager` from the JSON file named *filename*.

    See Also
    --------
    json_dump
    """
    with open(filename) as fh:
        return json.load(fh, object_hook=_json_decode)


class FontManager:
    """
    On import, the `FontManager` singleton instance creates a list of ttf and
    afm fonts and caches their `FontProperties`.  The `FontManager.findfont`
    method does a nearest neighbor search to find the font that most closely
    matches the specification.  If no good enough match is found, the default
    font is returned.

    plt-rs keeps the fonts and does the search; ``ttflist`` and ``afmlist``
    are its font lists.
    """
    # Increment this version number whenever the font cache data
    # format or behavior has changed and requires an existing font
    # cache files to be rebuilt.
    __version__ = '3.11.0'

    def __init__(self, size=None, weight='normal'):
        self._version = self.__version__
        self.__default_weight = weight
        self.default_size = size
        self.defaultFamily = {
            'ttf': 'DejaVu Sans',
            'afm': 'Helvetica'}

    @staticmethod
    def _entries(fontext):
        """The `FontEntry` of every font of the list *fontext*."""
        return [FontEntry(fname=fname, index=index, name=name, style=style,
                          variant=variant,
                          weight=int(weight) if weight.isdigit() else weight,
                          stretch=stretch)
                for fname, index, name, (style, variant, weight, stretch)
                in _font_manager.font_list(fontext)]

    @property
    def ttflist(self):
        """The TrueType fonts, as `FontEntry` objects."""
        return self._entries('ttf')

    @property
    def afmlist(self):
        """The Adobe Font Metrics of the PostScript core fonts, as
        `FontEntry` objects."""
        return self._entries('afm')

    def addfont(self, path):
        """
        Cache the properties of the font at *path* to make it available to the
        `FontManager`.  The type of font is inferred from the path suffix.

        Parameters
        ----------
        path : str or path-like
        """
        _font_manager.add_font(os.fsdecode(path))
        self._findfont_cached.cache_clear()

    @property
    def defaultFont(self):
        """The default font file of each font list."""
        return {ext: self.findfont(family, fontext=ext)
                for ext, family in self.defaultFamily.items()}

    def get_default_weight(self):
        """
        Return the default font weight.
        """
        return self.__default_weight

    @staticmethod
    def get_default_size():
        """
        Return the default font size.
        """
        return _rcParams()['font.size']

    def set_default_weight(self, weight):
        """
        Set the default font weight.  The initial value is 'normal'.
        """
        self.__default_weight = weight

    def score_weight(self, weight1, weight2):
        """
        Return a match score between *weight1* and *weight2*.

        The result is 0.0 if both weight1 and weight 2 are given as strings
        and have the same value.

        Otherwise, the result is the absolute value of the difference between
        the CSS numeric values of *weight1* and *weight2*, normalized between
        0.05 and 1.0.
        """
        # exact match of the weight names, e.g. weight1 == weight2 == "regular"
        if cbook._str_equal(weight1, weight2):
            return 0.0
        return _font_manager.score_weight(weight1, weight2)

    @staticmethod
    def _props(prop):
        """The native description of the font properties *prop*."""
        prop = FontProperties._from_any(prop)
        return (tuple(prop.get_family()),
                (prop.get_style(), prop.get_variant()),
                (prop.get_weight(), prop.get_stretch(), float(prop.get_size())),
                (prop.get_math_fontfamily(), prop.get_file()))

    def findfont(self, prop, fontext='ttf', directory=None,
                 fallback_to_default=True, rebuild_if_missing=True):
        """
        Find the path to the font file most closely matching the given font
        properties.

        Parameters
        ----------
        prop : str or `~matplotlib.font_manager.FontProperties`
            The font properties to search for. This can be either a
            `.FontProperties` object or a string defining a
            `fontconfig patterns`_.

        fontext : {'ttf', 'afm'}, default: 'ttf'
            The extension of the font file:

            - 'ttf': TrueType and OpenType fonts (.ttf, .ttc, .otf)
            - 'afm': Adobe Font Metrics (.afm)

        directory : str, optional
            pltrs searches the fonts it knows: a directory is not supported.

        fallback_to_default : bool
            If True, will fall back to the default font family (usually
            "DejaVu Sans" or "Helvetica") if the first lookup hard-fails.

        rebuild_if_missing : bool
            Whether to rebuild the font cache and search again if the first
            match appears to point to a nonexisting font (i.e., the font cache
            contains outdated entries).

        Returns
        -------
        FontPath
            The filename of the best matching font, and its face index.
        """
        if directory is not None:
            raise NotImplementedError("pltrs: findfont(directory=...)")
        return self._findfont_cached(self._props(prop), fontext,
                                     fallback_to_default, tuple(_rcParams()['font.family']))

    @lru_cache(1024)
    def _findfont_cached(self, props, fontext, fallback_to_default, family):
        """The `FontPath` the search finds (cached per properties and
        :rc:`font.family`)."""
        fname = _font_manager.find_font(props, fontext, fallback_to_default)
        index = next((entry.index for entry in self._entries(fontext)
                      if entry.fname == fname), 0)
        return FontPath(fname, index)

    def get_font_names(self):
        """Return the list of available fonts."""
        return list(_font_manager.font_names())

    def _find_fonts_by_props(self, prop, fontext='ttf', directory=None,
                             fallback_to_default=True, rebuild_if_missing=True):
        """
        Find the paths to the font files most closely matching the given
        properties: the best file of each family of *prop* that has one, in
        family order.
        """
        if fontext != 'ttf':
            raise NotImplementedError(
                f"pltrs: _find_fonts_by_props(fontext={fontext!r})")
        if directory is not None:
            raise NotImplementedError("pltrs: _find_fonts_by_props(directory=...)")
        return [FontPath(fname, 0) for fname in _font_manager.find_fonts(
            self._props(prop), fallback_to_default)]

    def _family_name(self, fpath):
        """The family name of the font at *fpath*."""
        return next((entry.name for entry in self.ttflist
                     if entry.fname == fpath), Path(fpath).stem)


fontManager = FontManager()
findfont = fontManager.findfont


def get_font_names():
    """Return the list of available fonts."""
    return fontManager.get_font_names()


def is_opentype_cff_font(filename):
    """
    Return whether the given font is a Postscript Compact Font Format Font
    embedded in an OpenType wrapper.  Used by the PostScript and PDF backends
    that cannot subset these fonts.
    """
    if os.path.splitext(filename)[1].lower() == '.otf':
        with open(filename, 'rb') as fd:
            return fd.read(4) == b"OTTO"
    else:
        return False
