//! x86-64 COFF object files, as far as the import library needs them:
//! sections with data and relocations, a symbol table and its string
//! table (Microsoft's PE/COFF specification, chapters 3 to 5).

/// `IMAGE_FILE_MACHINE_AMD64`.
pub(crate) const MACHINE_AMD64: u16 = 0x8664;

/// `IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_MEM_READ`.
pub(crate) const CODE: u32 = 0x6000_0020;
/// `IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_READ`.
pub(crate) const READ_ONLY_DATA: u32 = 0x4000_0040;
/// `IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_READ |
/// IMAGE_SCN_MEM_WRITE`.
pub(crate) const DATA: u32 = 0xC000_0040;

/// The `IMAGE_SCN_ALIGN_*` flag of an alignment of `bytes` (a power of
/// two up to 8192).
pub(crate) const fn align(bytes: u32) -> u32 {
    (bytes.trailing_zeros() + 1) << 20
}

/// How a relocation fills its 32-bit field.
#[derive(Clone, Copy, Debug)]
pub(crate) enum RelocationKind {
    /// `IMAGE_REL_AMD64_ADDR32NB`: the symbol's address relative to the
    /// image base (an RVA), plus the field's value.
    Rva = 3,
    /// `IMAGE_REL_AMD64_REL32`: the symbol's address relative to the end
    /// of the field, plus the field's value.
    Relative = 4,
}

/// A relocation of the 32-bit field at `offset` in its section.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Relocation {
    /// The field's offset in the section.
    pub(crate) offset: u32,
    /// The index of the symbol in the object's symbol table.
    pub(crate) symbol: u32,
    /// How the field is filled.
    pub(crate) kind: RelocationKind,
}

/// A section: its name (at most 8 bytes), `IMAGE_SCN_*` flags, contents
/// and relocations.
#[derive(Debug)]
pub(crate) struct Section {
    /// The name, such as `.text` or `.idata$2`.
    pub(crate) name: &'static str,
    /// The `IMAGE_SCN_*` flags.
    pub(crate) characteristics: u32,
    /// The contents.
    pub(crate) data: Vec<u8>,
    /// The relocations of the contents.
    pub(crate) relocations: Vec<Relocation>,
}

/// A symbol's storage class (`IMAGE_SYM_CLASS_*`).
#[derive(Clone, Copy, Debug)]
pub(crate) enum StorageClass {
    /// Visible to other objects (defined here, or undefined: section 0).
    External = 2,
    /// Local to the object.
    Static = 3,
    /// A section; undefined (section 0), the MSVC tools' reference to the
    /// first contribution to a section of that name in the same library.
    Section = 104,
}

/// A symbol of the symbol table.
#[derive(Debug)]
pub(crate) struct Symbol {
    /// The name.
    pub(crate) name: String,
    /// The offset in its section.
    pub(crate) value: u32,
    /// The 1-based index of its section, or 0 when undefined.
    pub(crate) section: u16,
    /// Whether it names a function (`IMAGE_SYM_DTYPE_FUNCTION`).
    pub(crate) function: bool,
    /// The storage class.
    pub(crate) class: StorageClass,
}

impl Symbol {
    /// A symbol of the given class defined at `value` in `section`.
    pub(crate) fn new(name: &str, section: u16, value: u32, class: StorageClass) -> Self {
        Self {
            name: name.to_owned(),
            value,
            section,
            function: false,
            class,
        }
    }

    /// An undefined external symbol.
    pub(crate) fn undefined(name: &str) -> Self {
        Self::new(name, 0, 0, StorageClass::External)
    }
}

/// The object file of `sections` and `symbols`, laid out as LLVM's and
/// Microsoft's tools write it: the header, the section headers, each
/// section's contents followed by its relocations, the symbol table and
/// the string table.
pub(crate) fn object(sections: &[Section], symbols: &[Symbol]) -> Vec<u8> {
    const HEADER: usize = 20;
    const SECTION_HEADER: usize = 40;
    const RELOCATION: usize = 10;
    let mut offset = HEADER + SECTION_HEADER * sections.len();
    let mut headers = Vec::new();
    let mut contents = Vec::new();
    for section in sections {
        let data_at = offset;
        offset += section.data.len();
        let relocations_at = if section.relocations.is_empty() {
            0
        } else {
            offset
        };
        offset += RELOCATION * section.relocations.len();
        headers.extend_from_slice(&name_field(section.name.as_bytes()));
        headers.extend_from_slice(&[0; 8]);
        put_u32(&mut headers, section.data.len());
        put_u32(&mut headers, data_at);
        put_u32(&mut headers, relocations_at);
        put_u32(&mut headers, 0);
        put_u16(&mut headers, section.relocations.len());
        put_u16(&mut headers, 0);
        headers.extend_from_slice(&section.characteristics.to_le_bytes());
        contents.extend_from_slice(&section.data);
        for relocation in &section.relocations {
            contents.extend_from_slice(&relocation.offset.to_le_bytes());
            contents.extend_from_slice(&relocation.symbol.to_le_bytes());
            contents.extend_from_slice(&(relocation.kind as u16).to_le_bytes());
        }
    }
    let mut out = Vec::with_capacity(offset + 64 * symbols.len());
    out.extend_from_slice(&MACHINE_AMD64.to_le_bytes());
    put_u16(&mut out, sections.len());
    put_u32(&mut out, 0);
    put_u32(&mut out, offset);
    put_u32(&mut out, symbols.len());
    put_u32(&mut out, 0);
    out.extend_from_slice(&headers);
    out.extend_from_slice(&contents);
    let mut strings = Vec::new();
    for symbol in symbols {
        let name = symbol.name.as_bytes();
        if name.len() > 8 {
            out.extend_from_slice(&[0; 4]);
            put_u32(&mut out, 4 + strings.len());
            strings.extend_from_slice(name);
            strings.push(0);
        } else {
            out.extend_from_slice(&name_field(name));
        }
        out.extend_from_slice(&symbol.value.to_le_bytes());
        out.extend_from_slice(&symbol.section.to_le_bytes());
        out.extend_from_slice(&(u16::from(symbol.function) << 5).to_le_bytes());
        out.push(symbol.class as u8);
        out.push(0);
    }
    put_u32(&mut out, 4 + strings.len());
    out.extend_from_slice(&strings);
    out
}

/// A name of at most 8 bytes, padded with zeros.
fn name_field(name: &[u8]) -> [u8; 8] {
    let mut field = [0; 8];
    field[..name.len()].copy_from_slice(name);
    field
}

/// Appends `value`, a size or offset in a small object, as a 16-bit
/// little-endian field.
pub(crate) fn put_u16(out: &mut Vec<u8>, value: usize) {
    let value = u16::try_from(value).expect("COFF field fits in 16 bits");
    out.extend_from_slice(&value.to_le_bytes());
}

/// Appends `value`, a size or offset in a small object, as a 32-bit
/// little-endian field.
pub(crate) fn put_u32(out: &mut Vec<u8>, value: usize) {
    let value = u32::try_from(value).expect("COFF field fits in 32 bits");
    out.extend_from_slice(&value.to_le_bytes());
}
