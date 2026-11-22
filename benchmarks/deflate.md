# DEFLATE: the port of zlib-ng

This page records the speed of the DEFLATE encoder
(`crates/pltrs-codec/src/deflate/encode`) before and after its port of
zlib-ng 2.3. For the same input, level and strategy, the encoder writes
the bytes of zlib-ng 2.3.3. `crates/pltrs-codec/tools/zlib_ng` checks
this. pltrs filters PNG rows as Pillow does, and compresses one row at
a time into 64 KiB buffers, with one `IDAT` chunk per buffer. The port
changed the compressed bytes of every PNG, PDF and image in an SVG, but
not pixels or document structure.

## DEFLATE alone

Level 6, one stream at a time, one core of MS-02 (AMD Ryzen 9 9955HX),
median of 31 runs:

| input | before | after | zlib-ng C, SIMD / generic |
|---|---:|---:|---:|
| 4.0 MB of PNG rows (`mpl_image/imshow_antialiased_double`) | 52.7 ms | 23.6 ms | 21.8 / 24.3 ms |
| 2.9 MB of PNG rows (`gallery_misc/topographic_hillshading`) | 41.8 ms | 28.7 ms | 27.8 / 29.7 ms |
| 3.4 MB of PNG rows (`gallery_images/plot_streamplot_1`) | 20.7 ms | 15.9 ms | 14.9 / 16.9 ms |
| 1.2 MB of PDF path text | 35.3 ms | 21.6 ms | |

PNG encoding with filtering, on the same machine:

| image | before | after |
|---|---:|---:|
| 4.0 MB | 54.8 ms | 25.0 ms |
| 2.9 MB | 43.5 ms | 30.0 ms |
| 3.4 MB | 22.9 ms | 17.5 ms |
| 4.0 MB at level 9 | 770 to 920 ms | 113 ms |

## The PNG and PDF workloads

`cargo xtask bench meshes/imshow medium/imshow meshes/pcolormesh
output/lissajous_1e5_pdf lines/plot_1e6_png lines/plot_1e6_pdf small/
--against` the pack of commit `b414751d` gives these values. Each value
is the new median over the old median.

| | MS-02 | Framework laptop (Ryzen AI 9 HX 370) |
|---|---:|---:|
| pltrs (Rust) warm, typically | −15.2% (20 faster, 1 within noise, 2 slower) | −15.2% (20 / 0 / 3) |
| pltrs (Python) warm, typically | −12.7% (22 / 1 / 0) | −13.2% (22 / 1 / 0) |
| pltrs (Rust) cold, typically | −14.2% | −9.5% |
| pltrs (Python) cold, typically | −0.8% | −3.0% |
| `output/lissajous_1e5_pdf`, Rust / Python warm | −38.5% / −37.7% | −39.1% / −38.3% |
| `small/*_png`, Rust warm | −15% to −36% | −16% to −35% |
| `meshes/imshow_60_*`, Rust warm | −11% to −13% | −8% to −12% |

The laptop column comes from an earlier build, in which each PDF stream
had a separate compressor. zlib-ng allocates and clears 300 KiB of tables per stream,
and a small PDF has about thirty streams. Thus `small/line_pdf` was 23%
slower in pltrs (Rust). Now the streams of a PDF share one compressor,
and `small/line_pdf` is 10.8% faster on MS-02.

Two workloads are slower in pltrs (Rust) and faster in pltrs (Python)
on both machines. On MS-02, `meshes/imshow_4000_antialiased` is +5.3%
and −8.4%, and `meshes/imshow_2000_bicubic_colorbar` is +9.7% and
−10.0%. Their PNG encoding is faster (13.3 ms to 10.8 ms), with as many page
faults. Stack samples put the difference in unchanged
code (resampling, normalization, colormap lookup). The probable cause is
how the compiler inlines and places that code.
