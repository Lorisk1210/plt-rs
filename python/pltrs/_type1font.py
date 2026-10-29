"""
A class representing a Type 1 font (matplotlib's ``_type1font``).

This version reads pfa and pfb files and splits them for embedding in
pdf files. It also supports SlantFont and ExtendFont transformations,
similarly to pdfTeX and friends.

Usage::

    font = Type1Font(filename)
    clear_part, encrypted_part, finale = font.parts
    slanted_font = font.transform({'slant': 0.167})
    extended_font = font.transform({'extend': 1.2})
    subset_font = font.subset([ord(c) for c in 'Hello World'], 'ABCDEF+')

pltrs (Rust) reads, rewrites and subsets the fonts (pltrs-font's
``Type1Font``) and tokenizes their PostScript; this module keeps
matplotlib's classes and signatures.
"""

from . import _api
from ._native import type1font as _type1font


class _Token:
    """
    A token in a PostScript stream.

    Attributes
    ----------
    kind : str
        Description of the token (for debugging or testing): ``'name'``,
        ``'boolean'``, ``'keyword'``, ``'delimiter'``, ``'whitespace'``,
        ``'string'``, ``'binary'`` or ``'number'``.
    pos : int
        Position, i.e. offset from the beginning of the data.
    raw : str or bytes
        Raw text of the token (bytes for a binary token).
    """
    __slots__ = ('kind', 'pos', 'raw', '_value')

    def __init__(self, kind, pos, raw, value):
        self.kind = kind
        self.pos = pos
        self.raw = raw if kind == 'binary' else raw.decode('ascii', 'replace')
        self._value = value

    def __str__(self):
        return f"<{self.kind} {self.raw} @{self.pos}>"

    def endpos(self):
        """Position one past the end of the token."""
        return self.pos + len(self.raw)

    def is_keyword(self, *names):
        """Is this a name token with one of the names?"""
        return self.kind == 'keyword' and self.raw in names

    def is_slash_name(self):
        """Is this a name token that starts with a slash?"""
        return self.kind == 'name' and self.raw.startswith('/')

    def is_delim(self):
        """Is this a delimiter token?"""
        return self.kind == 'delimiter'

    def is_number(self):
        """Is this a number token?"""
        return self.kind == 'number'

    def value(self):
        """The value of the token: numbers, booleans and strings decoded,
        names without their slash, binary runs without their separator."""
        return self._value


def _tokenize(data: bytes, skip_ws: bool):
    """
    A generator that produces _Token instances from Type-1 font code.

    The consumer of the _tokenize generator may send an integer to the
    tokenizer to indicate that the next token should be _BinaryToken of the
    given length.

    Parameters
    ----------
    data : bytes
        The data of the font to tokenize.

    skip_ws : bool
        If true, the generator will drop any _WhitespaceTokens from the output.
    """
    stream = _type1font.tokenizer(data, skip_ws)
    binary = None
    while (token := _type1font.next_token(stream, binary)) is not None:
        binary = yield _Token(*token)


class Type1Font:
    """
    A class representing a Type-1 font, for use by backends.

    Attributes
    ----------
    parts : tuple
        A 3-tuple of the cleartext part, the encrypted part, and the finale of
        zeros.

    decrypted : bytes
        The decrypted form of ``parts[1]``.

    prop : dict[str, Any]
        A dictionary of font properties. Noteworthy keys include:

        - FontName: PostScript name of the font
        - Encoding: dict from numeric codes to glyph names
        - FontMatrix: bytes object encoding a matrix
        - UniqueID: optional font identifier, dropped when modifying the font
        - CharStrings: dict from glyph names to byte code
        - Subrs: array of byte code subroutines
        - OtherSubrs: bytes object encoding some PostScript code
    """
    __slots__ = ('parts', 'decrypted', 'prop', '_pos', '_abbr', '_native')
    # the _pos dict contains (begin, end) indices to parts[0] + decrypted
    # so that they can be replaced when transforming the font;
    # but since sometimes a definition appears in both parts[0] and decrypted,
    # _pos[name] is an array of such pairs
    #
    # _abbr maps three standard abbreviations to their particular names in
    # this font (e.g. 'RD' is named '-|' in some fonts)

    def __init__(self, input):
        """
        Initialize a Type-1 font.

        Parameters
        ----------
        input : str or 3-tuple
            Either a pfb file name, or a 3-tuple of already-decoded Type-1
            font `~.Type1Font.parts`.
        """
        if isinstance(input, tuple) and len(input) == 3:
            native = _type1font.from_parts(*input)
        else:
            with open(input, 'rb') as file:
                native = _type1font.read(file.read())
        self._set_native(native)

    @classmethod
    def _from_native(cls, native):
        """The font of a native font (what the rewriting methods return)."""
        font = cls.__new__(cls)
        font._set_native(native)
        return font

    def _set_native(self, native):
        self._native = native
        self.parts = _type1font.parts(native)
        self.decrypted = _type1font.decrypted(native)
        self.prop = _type1font.prop(native)
        self._pos = dict(_type1font.positions(native))
        self._abbr = dict(_type1font.abbreviations(native))

    @staticmethod
    def _decrypt(ciphertext, key, ndiscard=4):
        """
        Decrypt ciphertext using the Type-1 font algorithm.

        The algorithm is described in Adobe's "Adobe Type 1 Font Format".
        The key argument is one of the strings
        'eexec' and 'charstring', which map to the key specified for the
        corresponding part of Type-1 fonts.

        The ndiscard argument should be an integer, usually 4.
        That number of bytes is discarded from the beginning of plaintext.
        """
        _api.check_in_list(['eexec', 'charstring'], key=key)
        return _type1font.decrypt(bytes(ciphertext), key, ndiscard)

    @staticmethod
    def _encrypt(plaintext, key, ndiscard=4):
        """
        Encrypt plaintext using the Type-1 font algorithm.

        The algorithm is described in Adobe's "Adobe Type 1 Font Format".
        The key argument is one of the strings
        'eexec' and 'charstring', which map to the key specified for the
        corresponding part of Type-1 fonts.

        The ndiscard argument should be an integer, usually 4. That
        number of bytes is prepended to the plaintext before encryption.
        This function prepends NUL bytes for reproducibility, even though
        the original algorithm uses random bytes, presumably to avoid
        cryptanalysis.
        """
        _api.check_in_list(['eexec', 'charstring'], key=key)
        return _type1font.encrypt(bytes(plaintext), key, ndiscard)

    def transform(self, effects):
        """
        Return a new font that is slanted and/or extended.

        Parameters
        ----------
        effects : dict
            A dict with optional entries:

            - 'slant' : float, default: 0
                Tangent of the angle that the font is to be slanted to the
                right. Negative values slant to the left.
            - 'extend' : float, default: 1
                Scaling factor for the font width. Values less than 1 condense
                the glyphs.

        Returns
        -------
        `Type1Font`
        """
        slant = effects.get('slant')
        extend = effects.get('extend')
        return Type1Font._from_native(_type1font.transform(
            self._native,
            None if slant is None else float(slant),
            None if extend is None else float(extend)))

    def with_encoding(self, encoding):
        """
        Change the encoding of the font.

        Parameters
        ----------
        encoding : dict
            A dictionary mapping character codes to glyph names.

        Returns
        -------
        `Type1Font`
        """
        return Type1Font._from_native(_type1font.with_encoding(
            self._native, [(int(code), str(glyph))
                           for code, glyph in encoding.items()]))

    def subset(self, characters, name_prefix):
        """
        Return a new font that only defines the given characters.

        Parameters
        ----------
        characters : sequence of bytes
            The subset of characters to include. These are indices into the
            font's encoding array. The encoding array of a Type-1 font can
            only include 256 characters, but other glyphs may be accessed
            via the seac operator.
        name_prefix : str
            Prefix to prepend to the font name.

        Returns
        -------
        `Type1Font`
        """
        return type(self)._from_native(_type1font.subset(
            self._native, sorted({int(code) for code in characters}),
            name_prefix))
