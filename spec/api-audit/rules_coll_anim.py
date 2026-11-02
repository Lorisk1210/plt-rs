"""Rules for collections, animation, font_manager and ticker."""
from rules_base import cov, diff, na, miss, ALIAS, CHANGE_TRACKING

C = r"matplotlib\.collections:"
AN = r"matplotlib\.animation:"
F = r"matplotlib\.font_manager:"
TK = r"matplotlib\.ticker:"
WRITER = diff("FuncAnimation::save / save_movie / to_gif / to_apng / to_webp / to_jshtml", "frames are encoded by FuncAnimation; a manual loop renders frames with Figure::to_png")

RULES = [
    (C + r"Collection\.get_transforms", na("per-item transforms are internal to the collection kinds")),
    (C + r"Collection\.get_fill", cov("Collection::style (CollectionStyle::facecolors)")),
    (C + r"Collection\.update_from", na("handles share state: there is nothing to copy (conventions)")),
    (C + r"Collection\.(get_aa|set_aa|get_ec|set_ec|get_fc|set_fc|get_ls|set_ls|get_lw|set_lw|get_dashes|set_dashes|get_transOffset|set_transOffset)", ALIAS),
    (C + r"Collection\.(get_antialiaseds|set_antialiaseds)", cov("Collection::set_antialiased / Collection::style")),
    (C + r"LineCollection\.set_verts", ALIAS),
    (C + r"LineCollection\.set_color", cov("Collection::set_edgecolor")),
    (C + r"LineCollection\.get_gapcolor", cov("Collection::style (CollectionStyle::gapcolors)")),
    (C + r"PolyCollection\.(set_verts|set_verts_and_codes)", miss("replacing the polygons of a collection")),
    (C + r"Collection\.set_paths", miss("replacing the paths of a collection")),
    (C + r"RegularPolyCollection\.(get_numsides|get_rotation)", diff("CollectionSpec::regular_polygons", "given when the collection is made")),
    (C + r"TriMesh\.convert_mesh_to_paths", cov("Collection::paths")),
    (C + r"_ScalarMappable\.changed", CHANGE_TRACKING),
    (C + r"_ColorizerInterface\.(autoscale|autoscale_None)", cov("Norm::autoscale / Norm::autoscale_none (Collection::norm, Collection::set_norm)")),
    (C + r"_ColorizerInterface\.colorbar", miss("the colorbar drawn for a mappable, from the mappable")),
    (r"matplotlib\.image:AxesImage\.callbacks", CHANGE_TRACKING),
    (AN + r"(AbstractMovieWriter|FileMovieWriter|MovieWriter|HTMLWriter|PillowWriter)\.(setup|grab_frame|finish|saving|frame_size)", WRITER),
    (AN + r"(Animation|FuncAnimation)\.(new_frame_seq|new_saved_frame_seq)", cov("Frames")),
    (AN + r"Animation\.(pause|resume)", miss("pausing and resuming an animation playing in a window")),
    (AN + r"FFMpegBase\.output_args", cov("MovieOptions::extra_args")),
    (AN + r"FileMovieWriter\.frame_format", cov("RcParams::animation_frame_format")),
    (AN + r".*\.(isAvailable|bin_path)", diff("RcParams::animation_ffmpeg_path", "only ffmpeg is external; save_movie reports when it cannot start")),
    (AN + r"MovieWriterRegistry\..*", diff("FuncAnimation::save", "the writer follows the file extension; no registry")),
    (F + r"FontEntry\.size", na("pltrs lists scalable fonts only")),
    (F + r"FontManager\.defaultFont", cov("FontManager::bundled")),
    (F + r"FontManager\.(get_default_weight|get_default_size|set_default_weight)", diff("RcParams::font_weight / RcParams::font_size", "defaults come from rcParams")),
    (F + r"FontManager\.(score_family|score_style|score_variant|score_stretch|score_size)", na("scoring internals of find_font (score_weight is public for tests)")),
    (F + r"FontPath\.path", cov("FontListEntry::file")),
    (F + r"FontPath\.face_index", cov("FontListEntry::index")),
    (F + r"FontProperties\.(get_family|get_name|set_family|set_name)", cov("FontProperties::families (public field)")),
    (F + r"FontProperties\.copy", cov("FontProperties (Clone)")),
    (F + r"FontProperties\.(get_slant|set_slant)", ALIAS),
    (F + r"FontProperties\.get_size_in_points", cov("FontProperties::size (public field)")),
    (TK + r".*\.set_params", diff("with_* builders of each locator", "locators are values: build a new one and set it on the axis")),
    (TK + r"(FixedFormatter|FuncFormatter)\.set_offset_string", cov("FixedFormatter / FuncFormatter with_offset_string")),
    (TK + r"Formatter\.fix_minus", cov("TickerSettings::fix_minus")),
    (TK + r"Locator\.raise_if_exceeds", cov("ticker::exceeds_max_ticks")),
    (TK + r"LogFormatter\.set_label_minor", cov("LogFormatter::with_label_only_base")),
    (TK + r"EngFormatter\.format_eng", cov("EngFormatter::format_data")),
    (TK + r"PercentFormatter\.convert_to_pct", cov("PercentFormatter::format_pct")),
    (TK + r"ScalarFormatter\.(get_useOffset|set_useOffset|useOffset)", cov("ScalarFormatter::use_offset / ScalarFormatter::with_use_offset")),
    (TK + r"ScalarFormatter\.(get_useMathText|set_useMathText|useMathText)", cov("ScalarFormatter::use_mathtext / ScalarFormatter::with_use_mathtext")),
    (TK + r"ScalarFormatter\.(get_useLocale|set_useLocale|useLocale)", na("pltrs does not read the process locale")),
]
