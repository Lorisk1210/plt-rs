"""
Writing a clip's frames to a file: H.264 MP4 through ffmpeg (on ``PATH``,
else the one the ``imageio_ffmpeg`` package brings), with the settings of
the user's own comparison project (CRF 18, yuv420p, faststart); without
either, an animated WebP (or GIF) through Pillow, at half the size and
frame rate.

Frames go to ffmpeg through a pipe, so that nothing but the clip is left
on disk.
"""

import shutil
import subprocess
from dataclasses import dataclass

from PIL import Image, features

from frames import FPS, H, W


@dataclass(frozen=True)
class Encoder:
    """How clips are written."""

    #: What the tool says it uses.
    name: str
    #: The ffmpeg executable, or None for Pillow.
    ffmpeg: str | None
    #: The clip's file suffix, frame rate and size.
    suffix: str
    fps: int
    size: tuple

    def write(self, frames, path):
        """Writes *frames* (RGB images of the frame's full size, `fps` of
        them a second) to *path*."""
        if self.ffmpeg is not None:
            _write_with_ffmpeg(self.ffmpeg, frames, path)
        else:
            _write_with_pillow(frames, path, self.fps, self.size)


def find_encoder():
    """ffmpeg on ``PATH``, else imageio-ffmpeg's, else Pillow."""
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is not None:
        return Encoder(f"ffmpeg ({ffmpeg})", ffmpeg, ".mp4", FPS, (W, H))
    try:
        import imageio_ffmpeg
    except ImportError:
        return pillow_encoder()
    return Encoder("ffmpeg of imageio-ffmpeg", imageio_ffmpeg.get_ffmpeg_exe(), ".mp4", FPS, (W, H))


def pillow_encoder():
    """Pillow's animated WebP, or GIF where Pillow lacks WebP."""
    suffix = ".webp" if features.check("webp") else ".gif"
    return Encoder(f"Pillow (animated {suffix[1:].upper()}, half size and rate)", None, suffix, FPS // 2, (W // 2, H // 2))


def _write_with_ffmpeg(ffmpeg, frames, path):
    """Pipes *frames* into ffmpeg, which writes *path*."""
    command = [
        ffmpeg, "-y", "-loglevel", "error",
        "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-",
        "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18", "-movflags", "+faststart",
        str(path),
    ]
    encoder = subprocess.Popen(command, stdin=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        for frame in frames:
            encoder.stdin.write(frame.tobytes())
        encoder.stdin.close()
    except BrokenPipeError:
        # ffmpeg stopped early; its own message follows.
        pass
    errors = encoder.stderr.read().decode(errors="replace")
    if encoder.wait() != 0:
        raise RuntimeError(f"ffmpeg could not write {path}:\n{errors}")


def _write_with_pillow(frames, path, fps, size):
    """Writes *frames*, scaled to *size*, as an animated image."""
    images = [frame.resize(size, Image.Resampling.LANCZOS) for frame in frames]
    first, rest = images[0], images[1:]
    first.save(path, save_all=True, append_images=rest, duration=round(1000 / fps), loop=0)
