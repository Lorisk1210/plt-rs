# The type stub of matplotlib._type1font: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from __future__ import annotations
import binascii
import functools
import itertools
import logging
import re
import string
import struct
import typing as T
import numpy as np
from matplotlib.cbook import _format_approx
from . import _api
_log = logging.getLogger(__name__)

class _Token:
    __slots__ = ('pos', 'raw')
    kind = '?'

    def __init__(self, pos, raw):
        self.pos = pos
        self.raw = raw

    def __str__(self):
        ...

    def endpos(self):
        ...

    def is_keyword(self, *names):
        ...

    def is_slash_name(self):
        ...

    def is_delim(self):
        ...

    def is_number(self):
        ...

    def value(self):
        ...

class _NameToken(_Token):
    kind = 'name'

    def is_slash_name(self):
        ...

    def value(self):
        ...

class _BooleanToken(_Token):
    kind = 'boolean'

    def value(self):
        ...

class _KeywordToken(_Token):
    kind = 'keyword'

    def is_keyword(self, *names):
        ...

class _DelimiterToken(_Token):
    kind = 'delimiter'

    def is_delim(self):
        ...

    def opposite(self):
        ...

class _WhitespaceToken(_Token):
    kind = 'whitespace'

class _StringToken(_Token):
    kind = 'string'
    _escapes_re = re.compile('\\\\([\\\\()nrtbf]|[0-7]{1,3})')
    _replacements = {'\\': '\\', '(': '(', ')': ')', 'n': '\n', 'r': '\r', 't': '\t', 'b': '\x08', 'f': '\x0c'}
    _ws_re = re.compile('[\x00\t\r\x0c\n ]')

    @classmethod
    def _escape(cls, match):
        ...

    @functools.lru_cache
    def value(self):
        ...

class _BinaryToken(_Token):
    kind = 'binary'

    def value(self):
        ...

class _NumberToken(_Token):
    kind = 'number'

    def is_number(self):
        ...

    def value(self):
        ...

def _tokenize(data: bytes, skip_ws: bool) -> T.Generator[_Token, int, None]:
    ...

class _BalancedExpression(_Token):
    pass

def _expression(initial, tokens, data):
    ...

class Type1Font:
    __slots__ = ('parts', 'decrypted', 'prop', '_pos', '_abbr')

    def __init__(self, input):
        self.decrypted = self._decrypt(self.parts[1], 'eexec')
        self._abbr = {'RD': 'RD', 'ND': 'ND', 'NP': 'NP'}
        self.parts = input
        self.parts = self._split(data)

    def _read(self, file):
        ...

    def _split(self, data):
        ...

    @staticmethod
    def _decrypt(ciphertext, key, ndiscard=4):
        ...

    @staticmethod
    def _encrypt(plaintext, key, ndiscard=4):
        ...

    def _parse(self):
        self.prop = prop
        self._pos = pos
        self._abbr['ND'] = key
        self._abbr['NP'] = key
        self._abbr['RD'] = key

    def _parse_subrs(self, tokens, _data):
        ...

    def _parse_charstrings(self, tokens, _data):
        ...

    @staticmethod
    def _parse_encoding(tokens, _data):
        ...

    def _parse_othersubrs(self, tokens, data):
        ...

    def transform(self, effects):
        ...

    def with_encoding(self, encoding):
        ...

    def _replace(self, replacements):
        ...

    def subset(self, characters, name_prefix):
        ...

    @staticmethod
    def _charstring_tokens(data):
        ...

    def _postscript_encoding(self, encoding):
        ...

    def _subset_charstrings(self, glyphs):
        ...

    def _subset_subrs(self, indices):
        ...

class _CharstringSimulator:
    __slots__ = ('font', 'buildchar_stack', 'postscript_stack', 'glyphs', 'subrs')

    def __init__(self, font):
        self.font = font
        self.buildchar_stack = []
        self.postscript_stack = []
        self.glyphs = set()
        self.subrs = set()

    def run(self, glyph_or_subr):
        ...

    def _step(self, opcode):
        ...
_StandardEncoding = {**{ord(letter): letter for letter in string.ascii_letters}, 0: '.notdef', 32: 'space', 33: 'exclam', 34: 'quotedbl', 35: 'numbersign', 36: 'dollar', 37: 'percent', 38: 'ampersand', 39: 'quoteright', 40: 'parenleft', 41: 'parenright', 42: 'asterisk', 43: 'plus', 44: 'comma', 45: 'hyphen', 46: 'period', 47: 'slash', 48: 'zero', 49: 'one', 50: 'two', 51: 'three', 52: 'four', 53: 'five', 54: 'six', 55: 'seven', 56: 'eight', 57: 'nine', 58: 'colon', 59: 'semicolon', 60: 'less', 61: 'equal', 62: 'greater', 63: 'question', 64: 'at', 91: 'bracketleft', 92: 'backslash', 93: 'bracketright', 94: 'asciicircum', 95: 'underscore', 96: 'quoteleft', 123: 'braceleft', 124: 'bar', 125: 'braceright', 126: 'asciitilde', 161: 'exclamdown', 162: 'cent', 163: 'sterling', 164: 'fraction', 165: 'yen', 166: 'florin', 167: 'section', 168: 'currency', 169: 'quotesingle', 170: 'quotedblleft', 171: 'guillemotleft', 172: 'guilsinglleft', 173: 'guilsinglright', 174: 'fi', 175: 'fl', 177: 'endash', 178: 'dagger', 179: 'daggerdbl', 180: 'periodcentered', 182: 'paragraph', 183: 'bullet', 184: 'quotesinglbase', 185: 'quotedblbase', 186: 'quotedblright', 187: 'guillemotright', 188: 'ellipsis', 189: 'perthousand', 191: 'questiondown', 193: 'grave', 194: 'acute', 195: 'circumflex', 196: 'tilde', 197: 'macron', 198: 'breve', 199: 'dotaccent', 200: 'dieresis', 202: 'ring', 203: 'cedilla', 205: 'hungarumlaut', 206: 'ogonek', 207: 'caron', 208: 'emdash', 225: 'AE', 227: 'ordfeminine', 232: 'Lslash', 233: 'Oslash', 234: 'OE', 235: 'ordmasculine', 241: 'ae', 245: 'dotlessi', 248: 'lslash', 249: 'oslash', 250: 'oe', 251: 'germandbls'}
__all__ = ['binascii', 'functools', 'itertools', 'logging', 're', 'string', 'struct', 'T', 'np', 'Type1Font']
