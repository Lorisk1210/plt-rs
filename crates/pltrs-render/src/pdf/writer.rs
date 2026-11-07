//! The low-level file of matplotlib's `PdfFile`: object numbering,
//! indirect objects, streams (Flate-compressed at `pdf.compression`), the
//! cross-reference table and the trailer.

use pltrs_codec::CompressionLevel;
use pltrs_codec::deflate::Options;
use pltrs_codec::zlib::Compressor;

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
    /// The compressor of every stream, which keeps its tables between
    /// them.
    compressor: Compressor,
    stream: Option<OpenStream>,
    /// Where the cross-reference table starts, once written.
    startxref: Option<usize>,
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
            compressor: Compressor::new(),
            stream: None,
            startxref: None,
        }
    }

    /// The object of the open stream, if any.
    pub(super) fn open_stream(&self) -> Option<u32> {
        self.stream.as_ref().map(|stream| stream.id)
    }

    /// Reserves an object number (`reserveObject`).
    pub(super) fn reserve(&mut self) -> u32 {
        self.offsets.push(None);
        (self.offsets.len() - 1) as u32
    }

    /// Records that object `id` starts here (`recordXref`).
    pub(super) fn record(&mut self, id: u32) {
        if let Some(offset) = self.offsets.get_mut(id as usize) {
            *offset = Some(self.out.len());
        }
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
            Some(level) if !stream.png => {
                let mut packed = Vec::new();
                self.compressor
                    .compress_into(&stream.data, Options::new(level), &mut packed)?;
                packed
            }
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

    /// The first object reserved but not written yet, if any.
    pub(super) fn unwritten(&self) -> Option<usize> {
        self.offsets.iter().position(Option::is_none)
    }

    /// Writes the cross-reference table (`writeXref`).
    ///
    /// # Errors
    ///
    /// When an object was reserved but never written.
    pub(super) fn write_xref(&mut self) -> Result<(), VectorError> {
        if let Some(id) = self.unwritten() {
            return Err(VectorError::UnwrittenObject(id));
        }
        let start = self.out.len();
        let mut table = format!("xref\n0 {}\n", self.offsets.len()).into_bytes();
        for (i, offset) in self.offsets.iter().enumerate() {
            let offset = offset.unwrap_or_default();
            let (generation, kind) = if i == 0 { (65535, 'f') } else { (0, 'n') };
            table.extend(format!("{offset:010} {generation:05} {kind} \n").into_bytes());
        }
        self.out.extend(table);
        self.startxref = Some(start);
        Ok(())
    }

    /// Writes the trailer naming the catalog `root` and the document
    /// information `info` (`writeTrailer`), after the cross-reference
    /// table.
    pub(super) fn write_trailer(&mut self, root: u32, info: u32) -> Result<(), VectorError> {
        self.out.extend_from_slice(b"trailer\n");
        let trailer = Obj::dict([
            ("Size", Obj::Int(self.offsets.len() as i64)),
            ("Root", Obj::Ref(root)),
            ("Info", Obj::Ref(info)),
        ]);
        self.out.extend(trailer.repr()?);
        let start = self.startxref.unwrap_or(self.out.len());
        self.out
            .extend(format!("\nstartxref\n{start}\n%%EOF\n").into_bytes());
        Ok(())
    }

    /// The file written so far.
    #[cfg(test)]
    pub(super) fn bytes(&self) -> &[u8] {
        &self.out
    }

    /// The file written.
    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.out
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
        w.write_xref().unwrap();
        w.write_trailer(root, info).unwrap();
        let file = String::from_utf8_lossy(&w.into_bytes()).into_owned();
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
            pltrs_codec::zlib::decompress(&file[start..end]).unwrap(),
            b"0 0 m\n"
        );
        assert!(String::from_utf8_lossy(&file).contains("/Filter /FlateDecode"));
    }
}
