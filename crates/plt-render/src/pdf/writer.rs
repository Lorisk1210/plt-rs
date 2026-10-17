//! The low-level file of matplotlib's `PdfFile`: object numbering,
//! indirect objects, streams (Flate-compressed at `pdf.compression`), the
//! cross-reference table and the trailer.

use plt_codec::CompressionLevel;

use super::object::{Obj, fill};
use crate::vector::VectorError;

/// A stream being written.
#[derive(Debug)]
struct OpenStream {
    id: u32,
    /// The object its length is written to, or `None` to inline it.
    length: Option<u32>,
    extra: Vec<(String, Obj)>,
    /// Data already PNG-encoded (stored without further compression).
    png: bool,
    data: Vec<u8>,
}

/// Assembles a PDF file in memory.
#[derive(Debug)]
pub(super) struct PdfWriter {
    out: Vec<u8>,
    /// Offset of every object; index 0 is the free head.
    offsets: Vec<Option<usize>>,
    compression: Option<CompressionLevel>,
    stream: Option<OpenStream>,
}

impl PdfWriter {
    /// A file with the PDF 1.4 header; `compression` 0 stores streams.
    pub(super) fn new(compression: u8) -> Self {
        let mut out = b"%PDF-1.4\n".to_vec();
        // Eight-bit characters mark the file as binary.
        out.extend_from_slice(b"%\xac\xdc \xab\xba\n");
        Self {
            out,
            offsets: vec![Some(0)],
            compression: CompressionLevel::new(compression.min(9))
                .ok()
                .filter(|level| level.get() > 0),
            stream: None,
        }
    }

    /// Reserves an object number (`reserveObject`).
    pub(super) fn reserve(&mut self) -> u32 {
        self.offsets.push(None);
        (self.offsets.len() - 1) as u32
    }

    fn record(&mut self, id: u32) {
        self.offsets[id as usize] = Some(self.out.len());
    }

    /// Writes an indirect object (`writeObject`).
    pub(super) fn write_object(&mut self, id: u32, contents: &Obj) -> Result<(), VectorError> {
        self.record(id);
        self.out.extend(format!("{id} 0 obj\n").into_bytes());
        self.out.extend(contents.repr()?);
        self.out.extend_from_slice(b"\nendobj\n");
        Ok(())
    }

    /// Appends bytes to the open stream (or the file when none is open).
    pub(super) fn write(&mut self, data: &[u8]) {
        match &mut self.stream {
            Some(stream) => stream.data.extend_from_slice(data),
            None => self.out.extend_from_slice(data),
        }
    }

    /// `output`: objects joined by `_fill`, then a newline.
    pub(super) fn output(&mut self, objects: &[Obj]) -> Result<(), VectorError> {
        let parts = objects
            .iter()
            .map(Obj::repr)
            .collect::<Result<Vec<_>, _>>()?;
        let mut line = fill(&parts);
        line.push(b'\n');
        self.write(&line);
        Ok(())
    }

    /// Opens a stream (`beginStream`); `length` is the object receiving
    /// its length, `png` marks PNG-predicted data with those parameters.
    pub(super) fn begin_stream(
        &mut self,
        id: u32,
        length: Option<u32>,
        mut extra: Vec<(String, Obj)>,
        png: Option<Obj>,
    ) {
        let png = png.is_some_and(|parms| {
            extra.push(("Filter".to_owned(), Obj::name("FlateDecode")));
            extra.push(("DecodeParms".to_owned(), parms));
            true
        });
        self.stream = Some(OpenStream {
            id,
            length,
            extra,
            png,
            data: Vec::new(),
        });
    }

    /// Closes the open stream (`endStream`), compressing its data.
    pub(super) fn end_stream(&mut self) -> Result<(), VectorError> {
        let Some(stream) = self.stream.take() else {
            return Ok(());
        };
        let data = match self.compression {
            Some(level) if !stream.png => plt_codec::zlib::compress(&stream.data, level),
            _ => stream.data,
        };
        let mut dict = stream.extra;
        let length = stream.length.map_or(Obj::Int(data.len() as i64), Obj::Ref);
        dict.push(("Length".to_owned(), length));
        if self.compression.is_some() && !dict.iter().any(|(k, _)| k == "Filter") {
            dict.push(("Filter".to_owned(), Obj::name("FlateDecode")));
        }
        self.record(stream.id);
        self.out
            .extend(format!("{} 0 obj\n", stream.id).into_bytes());
        self.out.extend(Obj::Dict(dict).repr()?);
        self.out.extend_from_slice(b"\nstream\n");
        self.out.extend_from_slice(&data);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
        if let Some(length) = stream.length {
            self.write_object(length, &Obj::Int(data.len() as i64))?;
        }
        Ok(())
    }

    /// Writes a whole stream with an inline length (`outputStream`).
    pub(super) fn output_stream(
        &mut self,
        id: u32,
        data: &[u8],
        extra: Vec<(String, Obj)>,
    ) -> Result<(), VectorError> {
        self.begin_stream(id, None, extra, None);
        self.write(data);
        self.end_stream()
    }

    /// Writes the cross-reference table and trailer and returns the file.
    pub(super) fn finish(mut self, root: u32, info: u32) -> Result<Vec<u8>, VectorError> {
        let start = self.out.len();
        self.out
            .extend(format!("xref\n0 {}\n", self.offsets.len()).into_bytes());
        for (i, offset) in self.offsets.iter().enumerate() {
            let offset = offset.expect("every reserved PDF object is written before the trailer");
            let (generation, kind) = if i == 0 { (65535, 'f') } else { (0, 'n') };
            self.out
                .extend(format!("{offset:010} {generation:05} {kind} \n").into_bytes());
        }
        self.out.extend_from_slice(b"trailer\n");
        let trailer = Obj::dict([
            ("Size", Obj::Int(self.offsets.len() as i64)),
            ("Root", Obj::Ref(root)),
            ("Info", Obj::Ref(info)),
        ]);
        self.out.extend(trailer.repr()?);
        self.out
            .extend(format!("\nstartxref\n{start}\n%%EOF\n").into_bytes());
        Ok(self.out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_streams_and_xref() {
        let mut w = PdfWriter::new(0);
        let root = w.reserve();
        let content = w.reserve();
        let length = w.reserve();
        w.write_object(root, &Obj::dict([("Type", Obj::name("Catalog"))]))
            .unwrap();
        w.begin_stream(content, Some(length), Vec::new(), None);
        w.output(&[Obj::Real(1.0), Obj::op("w")]).unwrap();
        w.end_stream().unwrap();
        let info = w.reserve();
        w.write_object(info, &Obj::Dict(Vec::new())).unwrap();
        let file = String::from_utf8_lossy(&w.finish(root, info).unwrap()).into_owned();
        assert!(file.contains("2 0 obj\n<< /Length 3 0 R >>\nstream\n1 w\n\nendstream"));
        assert!(file.contains("3 0 obj\n4\nendobj"));
        assert!(file.contains("xref\n0 5\n0000000000 65535 f \n"));
        assert!(file.ends_with("%%EOF\n"));
    }

    #[test]
    fn compressed_streams_inflate() {
        let mut w = PdfWriter::new(6);
        let id = w.reserve();
        w.output_stream(id, b"0 0 m\n", Vec::new()).unwrap();
        let file = w.out;
        let start = file.windows(7).position(|s| s == b"stream\n").unwrap() + 7;
        let end = file.windows(10).position(|s| s == b"\nendstream").unwrap();
        assert_eq!(
            plt_codec::zlib::decompress(&file[start..end]).unwrap(),
            b"0 0 m\n"
        );
        assert!(String::from_utf8_lossy(&file).contains("/Filter /FlateDecode"));
    }
}
