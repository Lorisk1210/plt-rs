//! Numbers in bulk: [`NdArray`], read from any buffer exporter (numpy
//! arrays of any numeric dtype, byte order and strides, `array.array`,
//! `memoryview`) or from nested sequences, and returned as numpy arrays.
//!
//! No numpy C API is involved. Reading goes through the buffer protocol;
//! writing copies the data into a `bytearray` that `numpy.frombuffer`
//! views, so the array Python gets owns its data and is writable.

use std::sync::OnceLock;

use super::containers::tuple;
use super::convert::{FromPy, IntoPy, items};
use super::error::{PyErr, PyResult};
use super::gil::Python;
use super::object::{Bound, Owned};
use crate::ffi;

/// One number read from a buffer, before it becomes an [`Element`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Scalar {
    /// A floating-point number.
    Float(f64),
    /// A signed integer.
    Int(i64),
    /// An unsigned integer.
    UInt(u64),
    /// A boolean.
    Bool(bool),
}

/// A type numbers in bulk convert to and from.
pub(crate) trait Element: Copy + 'static {
    /// numpy's dtype for arrays of it (`"<f8"`).
    const DTYPE: &'static str;
    /// A number read from a buffer as this type, if it fits.
    fn from_scalar(value: Scalar) -> Option<Self>;
    /// A number given as a Python object (in a nested sequence).
    fn from_object(object: &Bound<'_>) -> PyResult<Self>;
    /// Appends the value's little-endian bytes (numpy's layout here).
    fn write_le(self, bytes: &mut Vec<u8>);
    /// The value of its own little-endian bytes (`DTYPE`'s layout), as
    /// many as it takes.
    fn read_le(bytes: &[u8]) -> Self;
}

impl Element for f64 {
    const DTYPE: &'static str = "<f8";

    fn from_scalar(value: Scalar) -> Option<Self> {
        Some(match value {
            Scalar::Float(x) => x,
            Scalar::Int(x) => x as f64,
            Scalar::UInt(x) => x as f64,
            Scalar::Bool(x) => f64::from(u8::from(x)),
        })
    }

    fn from_object(object: &Bound<'_>) -> PyResult<Self> {
        object.extract()
    }

    fn write_le(self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.to_le_bytes());
    }

    fn read_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("an item of eight bytes"))
    }
}

impl Element for i64 {
    const DTYPE: &'static str = "<i8";

    fn from_scalar(value: Scalar) -> Option<Self> {
        match value {
            Scalar::Float(x) => (x.fract() == 0.0 && x.abs() < 9.2e18).then_some(x as i64),
            Scalar::Int(x) => Some(x),
            Scalar::UInt(x) => i64::try_from(x).ok(),
            Scalar::Bool(x) => Some(i64::from(x)),
        }
    }

    fn from_object(object: &Bound<'_>) -> PyResult<Self> {
        object.extract()
    }

    fn write_le(self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.to_le_bytes());
    }

    fn read_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("an item of eight bytes"))
    }
}

impl Element for u8 {
    const DTYPE: &'static str = "|u1";

    fn from_scalar(value: Scalar) -> Option<Self> {
        match value {
            Scalar::Float(x) => (x.fract() == 0.0 && (0.0..256.0).contains(&x)).then_some(x as u8),
            Scalar::Int(x) => u8::try_from(x).ok(),
            Scalar::UInt(x) => u8::try_from(x).ok(),
            Scalar::Bool(x) => Some(u8::from(x)),
        }
    }

    fn from_object(object: &Bound<'_>) -> PyResult<Self> {
        object.extract()
    }

    fn write_le(self, bytes: &mut Vec<u8>) {
        bytes.push(self);
    }

    fn read_le(bytes: &[u8]) -> Self {
        bytes[0]
    }
}

impl Element for bool {
    const DTYPE: &'static str = "|b1";

    fn from_scalar(value: Scalar) -> Option<Self> {
        Some(match value {
            Scalar::Float(x) => x != 0.0,
            Scalar::Int(x) => x != 0,
            Scalar::UInt(x) => x != 0,
            Scalar::Bool(x) => x,
        })
    }

    fn from_object(object: &Bound<'_>) -> PyResult<Self> {
        object.extract()
    }

    fn write_le(self, bytes: &mut Vec<u8>) {
        bytes.push(u8::from(self));
    }

    fn read_le(bytes: &[u8]) -> Self {
        bytes[0] != 0
    }
}

/// An n-dimensional array of numbers in row-major order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NdArray<T: Element = f64> {
    /// The numbers, the last dimension varying fastest.
    pub(crate) data: Vec<T>,
    /// The extent of each dimension (none for a scalar).
    pub(crate) shape: Vec<usize>,
}

impl<T: Element> NdArray<T> {
    /// A one-dimensional array.
    pub(crate) fn from_vec(data: Vec<T>) -> Self {
        let shape = vec![data.len()];
        Self { data, shape }
    }

    /// An array of `shape`, which must hold exactly `data.len()` numbers.
    pub(crate) fn with_shape(data: Vec<T>, shape: Vec<usize>) -> PyResult<Self> {
        if shape.iter().product::<usize>() == data.len() {
            Ok(Self { data, shape })
        } else {
            Err(PyErr::value_error(format!(
                "cannot reshape {} values into shape {shape:?}",
                data.len()
            )))
        }
    }

    /// The numbers of a scalar or one-dimensional array, refusing more
    /// dimensions.
    pub(crate) fn into_vec(self) -> PyResult<Vec<T>> {
        if self.shape.len() <= 1 {
            Ok(self.data)
        } else {
            Err(PyErr::value_error(format!(
                "expected a 1-D array, got {} dimensions",
                self.shape.len()
            )))
        }
    }
}

impl<'py, T: Element> FromPy<'py> for NdArray<T> {
    /// A buffer's numbers, or a scalar's, or those of nested sequences of
    /// equal lengths.
    fn from_py(object: &Bound<'py>) -> PyResult<Self> {
        match read_buffer(object)? {
            Some(array) => Ok(array),
            None => read_nested(object),
        }
    }
}

impl<T: Element> IntoPy for NdArray<T> {
    /// A numpy array owning a copy of the numbers.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let mut bytes = Vec::with_capacity(self.data.len() * std::mem::size_of::<T>());
        for value in self.data {
            value.write_le(&mut bytes);
        }
        let length = ffi::Py_ssize_t::try_from(bytes.len())
            .map_err(|_| PyErr::value_error("array too large"))?;
        // SAFETY: the GIL is held; `length` bytes at the pointer are
        // copied into a new `bytearray`.
        let storage = unsafe {
            Bound::from_owned_ptr(
                py,
                ffi::PyByteArray_FromStringAndSize(bytes.as_ptr().cast(), length),
            )
        }?;
        let flat = numpy_frombuffer(py)?.call(&[storage, T::DTYPE.into_py(py)?], &[])?;
        let shape = self
            .shape
            .into_iter()
            .map(|extent| extent.into_py(py))
            .collect::<PyResult<Vec<_>>>()?;
        flat.call_method("reshape", &[tuple(py, shape)?])
    }
}

/// `numpy.frombuffer`, imported once.
fn numpy_frombuffer(py: Python<'_>) -> PyResult<Bound<'_>> {
    static FROMBUFFER: OnceLock<Owned> = OnceLock::new();
    if let Some(function) = FROMBUFFER.get() {
        return Ok(function.bind(py));
    }
    // SAFETY: the GIL is held; a NUL-terminated name; a new reference.
    let numpy =
        unsafe { Bound::from_owned_ptr(py, ffi::PyImport_ImportModule(c"numpy".as_ptr())) }?;
    let function = numpy.getattr("frombuffer")?;
    Ok(FROMBUFFER.get_or_init(|| function.unbind()).bind(py))
}

/// How the items of a buffer are stored.
#[derive(Clone, Copy, Debug)]
struct Layout {
    kind: Kind,
    size: usize,
    big_endian: bool,
}

/// The kind of number of a buffer's items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Signed,
    Unsigned,
    Float,
    Bool,
}

impl Layout {
    /// The layout a `struct`-module format and item size describe, if it
    /// is a single number this module reads.
    fn parse(format: &str, size: usize) -> Option<Self> {
        let (big_endian, code) = match format.as_bytes() {
            [order @ (b'@' | b'=' | b'<' | b'>' | b'!'), code] => {
                (matches!(order, b'>' | b'!'), *code)
            }
            [code] => (false, *code),
            _ => return None,
        };
        let kind = match code {
            b'b' | b'h' | b'i' | b'l' | b'q' | b'n' => Kind::Signed,
            b'B' | b'H' | b'I' | b'L' | b'Q' | b'N' => Kind::Unsigned,
            // `g`, long double, is a double on Windows and on Apple's
            // ARM processors (wider elsewhere, where no size fits).
            b'e' | b'f' | b'd' | b'g' => Kind::Float,
            b'?' => Kind::Bool,
            _ => return None,
        };
        let sizes: &[usize] = match kind {
            Kind::Signed | Kind::Unsigned => &[1, 2, 4, 8],
            Kind::Float => &[2, 4, 8],
            Kind::Bool => &[1],
        };
        sizes.contains(&size).then_some(Self {
            kind,
            size,
            big_endian,
        })
    }

    /// numpy's dtype of the items, for those an [`Element`] is.
    fn dtype(self) -> Option<&'static str> {
        match (self.kind, self.size, self.big_endian) {
            (Kind::Float, 8, false) => Some(f64::DTYPE),
            (Kind::Signed, 8, false) => Some(i64::DTYPE),
            (Kind::Unsigned, 1, _) => Some(u8::DTYPE),
            (Kind::Bool, 1, _) => Some(bool::DTYPE),
            _ => None,
        }
    }

    /// Decodes one item from its bytes.
    fn decode(self, raw: &[u8]) -> Scalar {
        let mut bytes = [0_u8; 8];
        bytes[..self.size].copy_from_slice(raw);
        if self.big_endian {
            bytes[..self.size].reverse();
        }
        let bits = u64::from_le_bytes(bytes);
        let shift = 64 - 8 * self.size as u32;
        match (self.kind, self.size) {
            (Kind::Bool, _) => Scalar::Bool(bits != 0),
            (Kind::Unsigned, _) => Scalar::UInt(bits),
            (Kind::Signed, _) => Scalar::Int(((bits << shift) as i64) >> shift),
            (Kind::Float, 2) => Scalar::Float(half_to_f64(bits as u16)),
            (Kind::Float, 4) => Scalar::Float(f64::from(f32::from_bits(bits as u32))),
            (Kind::Float, _) => Scalar::Float(f64::from_bits(bits)),
        }
    }
}

/// An IEEE 754 half-precision number as an `f64`.
fn half_to_f64(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f64::from(bits & 0x3ff);
    sign * match exponent {
        0 => mantissa * 2f64.powi(-24),
        31 if mantissa == 0.0 => f64::INFINITY,
        31 => f64::NAN,
        _ => (1.0 + mantissa / 1024.0) * 2f64.powi(exponent - 15),
    }
}

/// A buffer view, released when dropped.
struct View {
    view: ffi::Py_buffer,
}

impl Drop for View {
    fn drop(&mut self) {
        // SAFETY: the view was filled by a successful `PyObject_GetBuffer`
        // and is released once, with the GIL held (as when it was taken).
        unsafe { ffi::PyBuffer_Release(&raw mut self.view) };
    }
}

/// The numbers of `object` through the buffer protocol, or `None` when it
/// exports no buffer.
fn read_buffer<T: Element>(object: &Bound<'_>) -> PyResult<Option<NdArray<T>>> {
    let py = object.py();
    // SAFETY: an all-zero `Py_buffer` is a valid "empty" value for the
    // exporter to fill (plain integers and null pointers).
    let empty = unsafe { std::mem::zeroed() };
    let mut view = View { view: empty };
    // SAFETY: a live object and a view for it to fill; on failure the
    // view stays untouched and an exception is set.
    let status = unsafe {
        ffi::PyObject_GetBuffer(object.as_ptr(), &raw mut view.view, ffi::PyBUF_RECORDS_RO)
    };
    if status != 0 {
        std::mem::forget(view);
        drop(PyErr::take(py));
        return Ok(None);
    }
    let raw = &view.view;
    // SAFETY: a filled view has a NUL-terminated format (requested with
    // `PyBUF_FORMAT`) and `ndim` entries in `shape` and `strides`.
    let (format, shape, strides) = unsafe {
        let format = std::ffi::CStr::from_ptr(raw.format)
            .to_string_lossy()
            .into_owned();
        let ndim = usize::try_from(raw.ndim).unwrap_or_default();
        let (shape, strides) = if ndim == 0 {
            (&[][..], &[][..])
        } else {
            (
                std::slice::from_raw_parts(raw.shape, ndim),
                std::slice::from_raw_parts(raw.strides, ndim),
            )
        };
        (format, shape.to_vec(), strides.to_vec())
    };
    let size = usize::try_from(raw.itemsize).unwrap_or_default();
    let layout = Layout::parse(&format, size).ok_or_else(|| {
        PyErr::type_error(format!(
            "cannot read numbers from a buffer of format {format:?}"
        ))
    })?;
    let shape: Vec<usize> = shape
        .iter()
        .map(|&extent| usize::try_from(extent).unwrap_or_default())
        .collect();
    let count = shape.iter().product::<usize>();
    let item = |bytes: &[u8]| {
        let scalar = layout.decode(bytes);
        T::from_scalar(scalar).ok_or_else(|| {
            PyErr::value_error(format!("{scalar:?} does not fit an array of {}", T::DTYPE))
        })
    };
    if is_row_major(&shape, &strides, size) {
        // SAFETY: a row-major buffer holds its `count` items of `size`
        // bytes one after another from `buf`, valid until the view is
        // released (when this function returns).
        let bytes = unsafe { std::slice::from_raw_parts(raw.buf.cast::<u8>(), count * size) };
        let data = if layout.dtype() == Some(T::DTYPE) {
            bytes.chunks_exact(size).map(T::read_le).collect()
        } else {
            bytes
                .chunks_exact(size)
                .map(item)
                .collect::<PyResult<_>>()?
        };
        return Ok(Some(NdArray { data, shape }));
    }
    let mut data = Vec::with_capacity(count);
    let mut index = vec![0_usize; shape.len()];
    for _ in 0..count {
        let offset: isize = index
            .iter()
            .zip(&strides)
            .map(|(&i, &stride)| i as isize * stride)
            .sum();
        // SAFETY: the exporter guarantees an item of `size` bytes at
        // `buf + Σ index·stride` for every index within `shape`, valid
        // until the view is released (after this loop).
        let bytes =
            unsafe { std::slice::from_raw_parts(raw.buf.cast::<u8>().offset(offset), size) };
        data.push(item(bytes)?);
        advance(&mut index, &shape);
    }
    Ok(Some(NdArray { data, shape }))
}

/// Whether `strides` lay out items of `size` bytes in row-major order,
/// one after another (extents of one or zero allow any stride).
fn is_row_major(shape: &[usize], strides: &[isize], size: usize) -> bool {
    let mut expected = size;
    for (&extent, &stride) in shape.iter().zip(strides).rev() {
        if extent > 1 && usize::try_from(stride).ok() != Some(expected) {
            return false;
        }
        expected *= extent;
    }
    true
}

/// Moves a row-major multi-index to the next position.
fn advance(index: &mut [usize], shape: &[usize]) {
    for (i, &extent) in index.iter_mut().zip(shape).rev() {
        *i += 1;
        if *i < extent {
            return;
        }
        *i = 0;
    }
}

/// The numbers of nested sequences of equal lengths, or of a scalar.
fn read_nested<T: Element>(object: &Bound<'_>) -> PyResult<NdArray<T>> {
    let scalar =
        object.has_type_flag(ffi::Py_TPFLAGS_UNICODE_SUBCLASS) || object.try_iter().is_err();
    if scalar {
        drop(PyErr::take(object.py()));
        return Ok(NdArray {
            data: vec![T::from_object(object)?],
            shape: Vec::new(),
        });
    }
    let rows: Vec<NdArray<T>> = items(object)?
        .iter()
        .map(read_nested)
        .collect::<PyResult<_>>()?;
    let inner = rows.first().map_or_else(Vec::new, |row| row.shape.clone());
    if rows.iter().any(|row| row.shape != inner) {
        return Err(PyErr::value_error(
            "setting an array element with a sequence: the nested sequences have different shapes",
        ));
    }
    let mut shape = vec![rows.len()];
    shape.extend(inner);
    let data = rows.into_iter().flat_map(|row| row.data).collect();
    Ok(NdArray { data, shape })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn layouts_decode_every_numeric_format() {
        let int16 = Layout::parse("<h", 2).unwrap();
        assert_eq!(int16.decode(&(-2_i16).to_le_bytes()), Scalar::Int(-2));
        let big = Layout::parse(">i", 4).unwrap();
        assert_eq!(big.decode(&7_i32.to_be_bytes()), Scalar::Int(7));
        let unsigned = Layout::parse("B", 1).unwrap();
        assert_eq!(unsigned.decode(&[200]), Scalar::UInt(200));
        let single = Layout::parse("=f", 4).unwrap();
        assert_eq!(single.decode(&1.5_f32.to_le_bytes()), Scalar::Float(1.5));
        assert_eq!(
            Layout::parse("?", 1).unwrap().decode(&[1]),
            Scalar::Bool(true)
        );
        assert_eq!(half_to_f64(0x3e00), 1.5);
        assert_eq!(half_to_f64(0xfc00), f64::NEG_INFINITY);
        assert!(half_to_f64(0x7e00).is_nan());
        assert!(Layout::parse("T{d:x:}", 8).is_none(), "structured items");
        assert!(Layout::parse("d", 3).is_none(), "a size no number has");
        let long_double = Layout::parse("g", 8).expect("a double, as on Windows");
        assert_eq!(
            long_double.decode(&0.25_f64.to_le_bytes()),
            Scalar::Float(0.25)
        );
        assert_eq!(long_double.dtype(), Some("<f8"));
        assert!(Layout::parse("g", 16).is_none(), "x87's extended precision");
    }

    #[test]
    fn items_in_an_elements_own_layout_read_as_it() {
        assert_eq!(Layout::parse("<d", 8).unwrap().dtype(), Some("<f8"));
        assert_eq!(Layout::parse(">d", 8).unwrap().dtype(), None, "swapped");
        assert_eq!(Layout::parse("f", 4).unwrap().dtype(), None, "single");
        assert_eq!(Layout::parse("q", 8).unwrap().dtype(), Some("<i8"));
        assert_eq!(Layout::parse("?", 1).unwrap().dtype(), Some("|b1"));
        assert_eq!(f64::read_le(&2.5_f64.to_le_bytes()), 2.5);
        assert_eq!(i64::read_le(&(-3_i64).to_le_bytes()), -3);
        assert!(bool::read_le(&[1]));
        assert_eq!(u8::read_le(&[7]), 7);
    }

    #[test]
    fn only_items_one_after_another_count_as_row_major() {
        assert!(is_row_major(&[2, 3], &[24, 8], 8));
        assert!(is_row_major(&[], &[], 8), "a scalar");
        assert!(is_row_major(&[1, 3], &[0, 8], 8), "an extent of one");
        assert!(is_row_major(&[0], &[16], 8), "no items");
        assert!(!is_row_major(&[3], &[16], 8), "every other item");
        assert!(!is_row_major(&[3], &[-8], 8), "reversed");
        assert!(!is_row_major(&[2, 3], &[8, 16], 8), "column-major");
    }

    #[test]
    fn buffers_are_read_with_their_shape_and_strides() {
        with_python(|py| {
            let flat = eval(py, "import array", "array.array('d', [1, 2, 3])");
            let flat = flat.extract::<NdArray>().unwrap();
            assert_eq!(flat, NdArray::from_vec(vec![1.0, 2.0, 3.0]));
            let shorts = eval(py, "import array", "array.array('h', [-1, 2])");
            assert_eq!(shorts.extract::<NdArray<i64>>().unwrap().data, [-1, 2]);
            let strided = eval(
                py,
                "import array",
                "memoryview(array.array('q', range(6)))[::2]",
            );
            assert_eq!(strided.extract::<NdArray>().unwrap().data, [0.0, 2.0, 4.0]);
            let reversed = eval(
                py,
                "import array",
                "memoryview(array.array('q', range(3)))[::-1]",
            );
            assert_eq!(reversed.extract::<NdArray>().unwrap().data, [2.0, 1.0, 0.0]);
            let grid = eval(
                py,
                "import array",
                "memoryview(array.array('d', range(6))).cast('B').cast('d', [2, 3])",
            );
            let grid = grid.extract::<NdArray>().unwrap();
            assert_eq!((grid.shape.as_slice(), grid.data[5]), (&[2, 3][..], 5.0));
            assert!(grid.into_vec().is_err(), "two dimensions are not a vector");
            let fractions = eval(py, "import array", "array.array('d', [0.5])");
            assert!(
                fractions.extract::<NdArray<i64>>().is_err(),
                "0.5 is no integer"
            );
        });
    }

    #[test]
    fn nested_sequences_and_scalars_are_arrays_too() {
        with_python(|py| {
            let nested = eval(py, "", "[[1, 2], (3, 4)]")
                .extract::<NdArray>()
                .unwrap();
            assert_eq!(nested.shape, [2, 2]);
            assert_eq!(nested.data, [1.0, 2.0, 3.0, 4.0]);
            let scalar = eval(py, "", "2.5").extract::<NdArray>().unwrap();
            assert_eq!(scalar.shape, [0_usize; 0]);
            assert_eq!(scalar.into_vec().unwrap(), [2.5]);
            let err = eval(py, "", "[[1, 2], [3]]")
                .extract::<NdArray>()
                .unwrap_err();
            assert_eq!(err.class_name(py), "ValueError");
            assert!(eval(py, "", "'text'").extract::<NdArray>().is_err());
            assert!(NdArray::with_shape(vec![1.0, 2.0], vec![3]).is_err());
        });
    }
}
