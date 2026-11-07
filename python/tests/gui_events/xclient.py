"""
A small X11 client speaking the protocol over the server's socket, with
nothing but the standard library: what the harness needs to drive another
program's windows as a user would, through the server's input path.

* `Display.find_window` finds a top-level window by its title,
  `Display.geometry` where it is on the screen;
* `Display.motion`, `Display.button` and `Display.key` fake input with the
  XTEST extension (the server delivers it as it delivers a real device's);
* `Display.keycode` maps a keysym to a key and the modifiers typing it,
  binding an unused key to keysyms the keyboard lacks (dead keys);
* `Display.configure` moves and resizes a window, and `Display.delete`
  asks it to close (``WM_DELETE_WINDOW``), as a window manager does;
* `Display.image` reads a window's pixels.
"""

import os
import socket
import struct

#: XTEST's event codes for ``FakeInput``.
KEY_PRESS, KEY_RELEASE, BUTTON_PRESS, BUTTON_RELEASE, MOTION = 2, 3, 4, 5, 6

#: Keysyms by name, for the keys the harness presses (X11's keysymdef.h).
KEYSYMS = {
    "BackSpace": 0xFF08, "Tab": 0xFF09, "Return": 0xFF0D, "Escape": 0xFF1B,
    "Delete": 0xFFFF, "Home": 0xFF50, "Left": 0xFF51, "Up": 0xFF52,
    "Right": 0xFF53, "Down": 0xFF54, "Prior": 0xFF55, "Next": 0xFF56,
    "End": 0xFF57, "Insert": 0xFF63, "Menu": 0xFF67, "Pause": 0xFF13,
    "KP_Enter": 0xFF8D,
    "Shift_L": 0xFFE1, "Shift_R": 0xFFE2, "Control_L": 0xFFE3,
    "Control_R": 0xFFE4, "Caps_Lock": 0xFFE5, "Alt_L": 0xFFE9,
    "Alt_R": 0xFFEA, "Super_L": 0xFFEB, "Super_R": 0xFFEC,
    "ISO_Level3_Shift": 0xFE03, "Multi_key": 0xFF20,
    "dead_grave": 0xFE50, "dead_acute": 0xFE51, "dead_circumflex": 0xFE52,
    "dead_tilde": 0xFE53, "dead_diaeresis": 0xFE57,
    **{f"F{n}": 0xFFBD + n for n in range(1, 25)},
    "space": 0x20,
}


class XError(Exception):
    """The server refused a request, or the connection failed."""


def _pad(n):
    """The padding after *n* bytes to the protocol's 4-byte units."""
    return -n % 4


def keysym_of(name):
    """The keysym of a key *name*: a name of `KEYSYMS`, or a character
    (Latin-1 characters are their own keysyms, others 0x01000000 above
    their code point)."""
    if name in KEYSYMS:
        return KEYSYMS[name]
    if len(name) != 1:
        raise KeyError(f"unknown key {name!r}")
    code = ord(name)
    return code if code < 0x100 else 0x01000000 | code


def _auth(number):
    """The MIT-MAGIC-COOKIE-1 of display *number* in the Xauthority file,
    as ``(name, data)``, or empty ones when there is none."""
    path = os.environ.get("XAUTHORITY") or os.path.expanduser("~/.Xauthority")
    try:
        with open(path, "rb") as file:
            data = file.read()
    except OSError:
        return b"", b""
    at = 0

    def field():
        nonlocal at
        (length,) = struct.unpack_from(">H", data, at)
        value = data[at + 2:at + 2 + length]
        at += 2 + length
        return value

    while at + 2 <= len(data):
        at += 2
        _address, display, name, cookie = field(), field(), field(), field()
        if display == str(number).encode() and name == b"MIT-MAGIC-COOKIE-1":
            return name, cookie
    return b"", b""


class Display:
    """A connection to the X server of *name* (default ``$DISPLAY``)."""

    def __init__(self, name=None):
        name = name if name is not None else os.environ.get("DISPLAY", "")
        host, _, rest = name.partition(":")
        if host not in ("", "unix"):
            raise XError(f"only local displays are supported: {name!r}")
        number = int(rest.partition(".")[0] or 0)
        self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            self._sock.connect(f"/tmp/.X11-unix/X{number}")
        except OSError as error:
            raise XError(f"cannot connect to {name!r}: {error}") from error
        self._sequence = 0
        self._atoms = {}
        self._setup(*_auth(number))
        self._xtest = self._extension(b"XTEST")
        if self._xtest is None:
            raise XError("the server lacks XTEST")
        self._read_keyboard()

    def close(self):
        """Close the connection."""
        self._sock.close()

    # The wire.

    def _recv(self, n):
        chunks = []
        while n:
            chunk = self._sock.recv(n)
            if not chunk:
                raise XError("the server closed the connection")
            chunks.append(chunk)
            n -= len(chunk)
        return b"".join(chunks)

    def _setup(self, auth_name, auth_data):
        self._sock.sendall(
            struct.pack("<BxHHHHxx", 0x6C, 11, 0, len(auth_name), len(auth_data))
            + auth_name + b"\0" * _pad(len(auth_name))
            + auth_data + b"\0" * _pad(len(auth_data)))
        status, _, _major, _minor, length = struct.unpack("<BBHHH", self._recv(8))
        body = self._recv(4 * length)
        if status != 1:
            reason = body[:body[0] if status == 0 else len(body)]
            raise XError(f"the server refused the connection: {reason!r}")
        (self._id_base, self._id_mask, vendor_length, formats) = (
            struct.unpack_from("<4xII4xH2xxB", body, 0))
        self.min_keycode, self.max_keycode = struct.unpack_from("<BB", body, 26)
        at = 32 + vendor_length + _pad(vendor_length) + 8 * formats
        (self.root, self.width, self.height) = (
            struct.unpack_from("<I16xHH", body, at))
        self._next_id = 0

    def _new_id(self):
        self._next_id += 1
        return self._id_base | (self._next_id & self._id_mask)

    def _send(self, opcode, data_byte, body):
        """Send a request; its sequence number."""
        length = 1 + (len(body) + _pad(len(body))) // 4
        self._sock.sendall(struct.pack("<BBH", opcode, data_byte, length)
                           + body + b"\0" * _pad(len(body)))
        self._sequence = (self._sequence + 1) & 0xFFFF
        return self._sequence

    def _reply(self, sequence):
        """The reply to request *sequence*, skipping events; raises
        `XError` for an error."""
        while True:
            head = self._recv(32)
            kind = head[0]
            if kind == 0:
                (code, seq, value, minor, major) = struct.unpack_from(
                    "<xBHIHB", head)
                if seq == sequence:
                    raise XError(f"X error {code} for request {major}.{minor} "
                                 f"(value {value:#x})")
                continue
            if kind == 1:
                (seq, extra) = struct.unpack_from("<2xHI", head)
                body = head + self._recv(4 * extra)
                if seq == sequence:
                    return body
                continue
            # An event: the harness selects none, but the server may send
            # some (MappingNotify) anyway.

    def _request(self, opcode, data_byte, body):
        return self._reply(self._send(opcode, data_byte, body))

    def sync(self):
        """Wait until the server has handled every request sent."""
        self._request(43, 0, b"")  # GetInputFocus

    # Atoms, properties and windows.

    def atom(self, name):
        """The atom named *name*, made if needed."""
        if name not in self._atoms:
            raw = name.encode()
            reply = self._request(16, 0, struct.pack("<H2x", len(raw)) + raw)
            self._atoms[name] = struct.unpack_from("<I", reply, 8)[0]
        return self._atoms[name]

    def property(self, window, name):
        """The bytes of *window*'s property *name*, or None."""
        reply = self._request(20, 0, struct.pack(
            "<IIIII", window, self.atom(name), 0, 0, 1 << 20))
        fmt, = struct.unpack_from("<B", reply, 1)
        kind, _after, count = struct.unpack_from("<III", reply, 8)
        if kind == 0:
            return None
        return reply[32:32 + count * max(fmt // 8, 1)]

    def children(self, window):
        """The children of *window*, bottom first."""
        reply = self._request(15, 0, struct.pack("<I", window))
        (count,) = struct.unpack_from("<H", reply, 16)
        return list(struct.unpack_from(f"<{count}I", reply, 32))

    def title(self, window):
        """*window*'s title (``_NET_WM_NAME``, else ``WM_NAME``), or None."""
        for name in ("_NET_WM_NAME", "WM_NAME"):
            value = self.property(window, name)
            if value is not None:
                return value.decode("utf-8", "replace")
        return None

    def is_viewable(self, window):
        """Whether *window* is mapped with all its ancestors."""
        reply = self._request(3, 0, struct.pack("<I", window))
        return reply[26] == 2

    def find_window(self, title):
        """The viewable top-level window titled *title*, or None (its
        frame's child, under a window manager)."""
        for window in self.children(self.root):
            for candidate in [window, *self.children(window)]:
                if (self.title(candidate) == title
                        and self.is_viewable(candidate)):
                    return candidate
        return None

    def geometry(self, window):
        """``(x, y, width, height)`` of *window*, its position from the
        screen's top-left corner."""
        reply = self._request(14, 0, struct.pack("<I", window))
        width, height = struct.unpack_from("<HH", reply, 16)
        reply = self._request(40, 0, struct.pack("<IIhh", window, self.root, 0, 0))
        x, y = struct.unpack_from("<hh", reply, 12)
        return x, y, width, height

    def configure(self, window, x=None, y=None, width=None, height=None):
        """Move and resize *window*, as a window manager does."""
        mask, values = 0, []
        for bit, value in [(1, x), (2, y), (4, width), (8, height)]:
            if value is not None:
                mask |= bit
                values.append(value & 0xFFFFFFFF)
        self._send(12, 0, struct.pack(f"<IH2x{len(values)}I", window, mask,
                                      *values))
        self.sync()

    def focus(self, window):
        """Give *window* the keyboard focus, as a window manager does when
        a window opens."""
        self._send(42, 1, struct.pack("<II", window, 0))  # SetInputFocus
        self.sync()

    def delete(self, window):
        """Ask *window* to close, as a window manager does when its close
        button is clicked (``WM_DELETE_WINDOW``)."""
        event = struct.pack("<BBHII5I", 33, 32, 0, window,
                            self.atom("WM_PROTOCOLS"),
                            self.atom("WM_DELETE_WINDOW"), 0, 0, 0, 0)
        self._send(25, 0, struct.pack("<II", window, 0) + event)
        self.sync()

    def image(self, window, x, y, width, height):
        """The pixels of a rectangle of *window* as RGB bytes, rows from the
        top (a 24-bit TrueColor server's little-endian ``ZPixmap``)."""
        reply = self._request(73, 2, struct.pack(
            "<IhhHHI", window, x, y, width, height, 0xFFFFFFFF))
        data = reply[32:]
        rgb = bytearray(3 * width * height)
        rgb[0::3] = data[2::4][:width * height]
        rgb[1::3] = data[1::4][:width * height]
        rgb[2::3] = data[0::4][:width * height]
        return bytes(rgb)

    # Extensions and input.

    def _extension(self, name):
        """The major opcode of extension *name*, or None."""
        reply = self._request(98, 0, struct.pack("<H2x", len(name)) + name)
        present, major = struct.unpack_from("<BB", reply, 8)
        return major if present else None

    def _fake(self, kind, detail, x=0, y=0):
        self._send(self._xtest, 2, struct.pack(
            "<BBxxII8xhh7xB", kind, detail, 0, self.root if kind == MOTION else 0,
            x, y, 0))

    def motion(self, x, y):
        """Move the pointer to *(x, y)* on the screen."""
        self._fake(MOTION, 0, x, y)
        self.sync()

    def button(self, number, pressed):
        """Press or release pointer button *number* (4 to 7: the wheel)."""
        self._fake(BUTTON_PRESS if pressed else BUTTON_RELEASE, number)
        self.sync()

    def key(self, keycode, pressed):
        """Press or release the key *keycode*."""
        self._fake(KEY_PRESS if pressed else KEY_RELEASE, keycode)
        self.sync()

    # The keyboard.

    def _read_keyboard(self):
        count = self.max_keycode - self.min_keycode + 1
        reply = self._request(101, 0, struct.pack("<BB2x", self.min_keycode, count))
        per = reply[1]
        syms = struct.unpack_from(f"<{per * count}I", reply, 32)
        self._keymap = {self.min_keycode + i: list(syms[per * i:per * (i + 1)])
                        for i in range(count)}
        self._per_keycode = per

    def keycode(self, keysym):
        """``(keycode, shifted)``: the key typing *keysym*, and whether
        Shift must be held for it. A keysym the keyboard lacks is bound to
        an unused key first, unshifted."""
        for level in (0, 1):
            for code, syms in self._keymap.items():
                if len(syms) > level and syms[level] == keysym:
                    return code, level == 1
        spare = next(code for code, syms in sorted(self._keymap.items(),
                                                     reverse=True)
                     if not any(syms))
        syms = [keysym, keysym] + [0] * (self._per_keycode - 2)
        self._send(100, 1, struct.pack(f"<BB2x{len(syms)}I", spare,
                                       self._per_keycode, *syms))
        self.sync()
        self._keymap[spare] = syms
        return spare, False
