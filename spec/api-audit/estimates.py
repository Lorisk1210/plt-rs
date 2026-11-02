"""What is still missing, with an estimate, and what the audits implemented."""

# Keyed by the note of the missing items (see rules*.py).
ESTIMATES = {
    "saving AVIF images (matplotlib saves them through Pillow; pltrs (Python) too)":
        "large (an AV1 intra-frame encoder, several thousand lines: transforms, prediction "
        "and the multi-symbol entropy coder, in a HEIF container); matplotlib has no encoder "
        "of its own either",
}

# (Rust items, what they give), by audit.
IMPLEMENTED = [
    ("2026-11-02", [
        ("Figure::tools / tool / set_tool_keys / active_toggle",
         "matplotlib's tool manager: `tools`, `get_tool`, `get_tool_keymap`, `update_keymap`, "
         "`active_toggle`; `remove_tool` switches a toggle off first"),
        ("Figure::on_tool_change / disconnect_tool_change / tool_message, ToolChange",
         "`toolmanager_connect`: tools added, removed and triggered, and tool messages, which "
         "the windows' status line shows"),
        ("MovieWriter (new, grab_frame, frame_size, finish)",
         "matplotlib's `MovieWriter` classes: frames of a figure the program draws itself, "
         "written as an animated PNG, GIF or WebP, an HTML page or an ffmpeg video"),
        ("Figure::to_pgf_pdf / to_pgf_png, PgfPages, PdfToPng",
         "the PGF backend's `print_pdf`, `print_png` and `PdfPages`: pictures typeset by "
         "`pgf.texsystem`, rasterised by pdftocairo or Ghostscript"),
        ("testing::convert / get_file_hash / comparable_formats",
         "comparing PDF and PostScript files through Ghostscript, SVG files through Inkscape "
         "and GIF files, as `matplotlib.testing.compare` does"),
        ("testing::save_diff_image / crop_to_same / make_test_filename",
         "the remaining helpers of `matplotlib.testing.compare`"),
        ("testing::image_comparison (ImageComparison), testing::remove_ticks_and_titles",
         "`@image_comparison` and `remove_ticks_and_titles` of `matplotlib.testing.decorators`"),
        ("TexManager::rgba (TexRgba)", "`TexManager.get_rgba`"),
    ]),
]
