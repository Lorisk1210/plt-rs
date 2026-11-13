# The type stub of matplotlib._fontconfig_pattern: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from functools import cache, lru_cache, partial
import re
from pyparsing import Group, Optional, ParseException, Regex, StringEnd, Suppress, ZeroOrMore, one_of
_family_punc = '\\\\\\-:,'
_family_unescape = partial(re.compile('\\\\(?=[%s])' % _family_punc).sub, '')
_family_escape = partial(re.compile('(?=[%s])' % _family_punc).sub, '\\\\')
_value_punc = '\\\\=_:,'
_value_unescape = partial(re.compile('\\\\(?=[%s])' % _value_punc).sub, '')
_value_escape = partial(re.compile('(?=[%s])' % _value_punc).sub, '\\\\')
_CONSTANTS = {'thin': ('weight', 'light'), 'extralight': ('weight', 'light'), 'ultralight': ('weight', 'light'), 'light': ('weight', 'light'), 'book': ('weight', 'book'), 'regular': ('weight', 'regular'), 'normal': ('weight', 'normal'), 'medium': ('weight', 'medium'), 'demibold': ('weight', 'demibold'), 'semibold': ('weight', 'semibold'), 'bold': ('weight', 'bold'), 'extrabold': ('weight', 'extra bold'), 'black': ('weight', 'black'), 'heavy': ('weight', 'heavy'), 'roman': ('slant', 'normal'), 'italic': ('slant', 'italic'), 'oblique': ('slant', 'oblique'), 'ultracondensed': ('width', 'ultra-condensed'), 'extracondensed': ('width', 'extra-condensed'), 'condensed': ('width', 'condensed'), 'semicondensed': ('width', 'semi-condensed'), 'expanded': ('width', 'expanded'), 'extraexpanded': ('width', 'extra-expanded'), 'ultraexpanded': ('width', 'ultra-expanded')}

@cache
def _make_fontconfig_parser():
    ...

@lru_cache
def parse_fontconfig_pattern(pattern):
    ...

def generate_fontconfig_pattern(d):
    ...
__all__ = ['cache', 'lru_cache', 'partial', 're', 'Group', 'Optional', 'ParseException', 'Regex', 'StringEnd', 'Suppress', 'ZeroOrMore', 'one_of', 'parse_fontconfig_pattern', 'generate_fontconfig_pattern']
