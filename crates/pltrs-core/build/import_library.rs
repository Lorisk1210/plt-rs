//! Import libraries of a DLL, as the MSVC tools write them (`lib /def:`)
//! and GNU ld reads them (Microsoft's PE/COFF specification, chapter 7
//! and the import library format; LLVM's `COFFImportFile.cpp` writes the
//! same bytes).
//!
//! The library is an `ar` archive of:
//! * the import descriptor, an object defining
//!   `__IMPORT_DESCRIPTOR_<dll>`: the DLL's entry of the import directory
//!   (`.idata$2`) and its name (`.idata$6`), pointing at the start of the
//!   DLL's import lookup and address tables (`.idata$4`, `.idata$5`);
//! * the null import descriptor, which ends the import directory;
//! * the null thunk, which ends the DLL's two tables;
//! * a short import object per function: the linker makes its entries of
//!   the two tables, its hint/name entry, the pointer `__imp_<name>` and
//!   the stub `<name>` that jumps through it;
//! * any other objects given (whose code may call `__imp_<name>`).
//!
//! The archive starts with the two symbol tables of COFF archives, which
//! map each symbol a member defines to the member, and the table of member
//! names longer than 15 bytes.

use crate::coff::{self, Relocation, RelocationKind, Section, StorageClass, Symbol};

/// A member of the archive: its name, contents and the symbols it defines.
#[derive(Debug)]
pub(crate) struct Member {
    /// The member's name.
    pub(crate) name: String,
    /// The member's contents.
    pub(crate) data: Vec<u8>,
    /// The external symbols the member defines.
    pub(crate) symbols: Vec<String>,
}

/// The import library of the functions `functions` of `dll`, followed by
/// the members `extra`.
pub(crate) fn write(dll: &str, functions: &[&str], extra: Vec<Member>) -> Vec<u8> {
    let stem = dll.rsplit_once('.').map_or(dll, |(stem, _)| stem);
    let mut members = vec![
        import_descriptor(dll, stem),
        null_import_descriptor(dll),
        null_thunk(dll, stem),
    ];
    members.extend(functions.iter().map(|name| short_import(dll, name)));
    members.extend(extra);
    archive(&members)
}

/// The symbol the null import descriptor defines.
const NULL_IMPORT_DESCRIPTOR: &str = "__NULL_IMPORT_DESCRIPTOR";

/// The symbol the null thunk of the DLL `stem` defines.
fn null_thunk_symbol(stem: &str) -> String {
    format!("\x7f{stem}_NULL_THUNK_DATA")
}

/// The import descriptor of `dll`, whose name without its extension is
/// `stem`.
fn import_descriptor(dll: &str, stem: &str) -> Member {
    let descriptor = format!("__IMPORT_DESCRIPTOR_{stem}");
    // The entry's fields: lookup table, time stamp, forwarder chain, name
    // and address table; the linker fills in the three RVAs.
    let entry = Section {
        name: ".idata$2",
        characteristics: coff::DATA | coff::align(4),
        data: vec![0; 20],
        relocations: [(12, 2), (0, 3), (16, 4)]
            .into_iter()
            .map(|(offset, symbol)| Relocation {
                offset,
                symbol,
                kind: RelocationKind::Rva,
            })
            .collect(),
    };
    let mut name = dll.as_bytes().to_vec();
    name.push(0);
    let name = Section {
        name: ".idata$6",
        characteristics: coff::DATA | coff::align(2),
        data: name,
        relocations: Vec::new(),
    };
    let symbols = [
        Symbol::new(&descriptor, 1, 0, StorageClass::External),
        Symbol::new(".idata$2", 1, 0, StorageClass::Section),
        Symbol::new(".idata$6", 2, 0, StorageClass::Static),
        Symbol::new(".idata$4", 0, 0, StorageClass::Section),
        Symbol::new(".idata$5", 0, 0, StorageClass::Section),
        Symbol::undefined(NULL_IMPORT_DESCRIPTOR),
        Symbol::undefined(&null_thunk_symbol(stem)),
    ];
    Member {
        name: dll.to_owned(),
        data: coff::object(&[entry, name], &symbols),
        symbols: vec![descriptor],
    }
}

/// The null import descriptor, the all-zero entry that ends the import
/// directory.
fn null_import_descriptor(dll: &str) -> Member {
    let entry = Section {
        name: ".idata$3",
        characteristics: coff::DATA | coff::align(4),
        data: vec![0; 20],
        relocations: Vec::new(),
    };
    let symbols = [Symbol::new(
        NULL_IMPORT_DESCRIPTOR,
        1,
        0,
        StorageClass::External,
    )];
    Member {
        name: dll.to_owned(),
        data: coff::object(&[entry], &symbols),
        symbols: vec![NULL_IMPORT_DESCRIPTOR.to_owned()],
    }
}

/// The null thunk of `dll`: the zero entries that end its import address
/// and lookup tables.
fn null_thunk(dll: &str, stem: &str) -> Member {
    let table_end = |name| Section {
        name,
        characteristics: coff::DATA | coff::align(8),
        data: vec![0; 8],
        relocations: Vec::new(),
    };
    let symbol = null_thunk_symbol(stem);
    let symbols = [Symbol::new(&symbol, 1, 0, StorageClass::External)];
    Member {
        name: dll.to_owned(),
        data: coff::object(&[table_end(".idata$5"), table_end(".idata$4")], &symbols),
        symbols: vec![symbol],
    }
}

/// The short import object of the function `name` of `dll`, imported by
/// name.
fn short_import(dll: &str, name: &str) -> Member {
    // IMPORT_OBJECT_CODE, with IMPORT_OBJECT_NAME: the symbol's name is
    // the name the DLL exports.
    const CODE_BY_NAME: u16 = 1 << 2;
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0xFFFFu16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&coff::MACHINE_AMD64.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    coff::put_u32(&mut data, name.len() + dll.len() + 2);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&CODE_BY_NAME.to_le_bytes());
    for text in [name, dll] {
        data.extend_from_slice(text.as_bytes());
        data.push(0);
    }
    Member {
        name: dll.to_owned(),
        data,
        symbols: vec![format!("__imp_{name}"), name.to_owned()],
    }
}

/// The length of a member header.
const HEADER: usize = 60;

/// The `ar` archive of `members`, with the two COFF symbol tables and
/// the long-name table first.
fn archive(members: &[Member]) -> Vec<u8> {
    let mut long_names = Vec::new();
    let names: Vec<String> = members
        .iter()
        .map(|member| {
            if member.name.len() <= 15 {
                format!("{}/", member.name)
            } else {
                let at = find(&long_names, &member.name).unwrap_or_else(|| {
                    let at = long_names.len();
                    long_names.extend_from_slice(member.name.as_bytes());
                    long_names.push(0);
                    at
                });
                format!("/{at}")
            }
        })
        .collect();
    let symbols: Vec<(&str, usize)> = members
        .iter()
        .enumerate()
        .flat_map(|(index, member)| member.symbols.iter().map(move |s| (s.as_str(), index)))
        .collect();
    let names_size: usize = symbols.iter().map(|(name, _)| name.len() + 1).sum();
    let first_size = 4 + 4 * symbols.len() + names_size;
    let second_size = 4 + 4 * members.len() + 4 + 2 * symbols.len() + names_size;
    let mut offset = 8 + HEADER + padded(first_size) + HEADER + padded(second_size);
    if !long_names.is_empty() {
        offset += HEADER + padded(long_names.len());
    }
    let offsets: Vec<usize> = members
        .iter()
        .map(|member| {
            let at = offset;
            offset += HEADER + padded(member.data.len());
            at
        })
        .collect();

    let mut first = Vec::with_capacity(first_size);
    first.extend_from_slice(&count(symbols.len()).to_be_bytes());
    for &(_, member) in &symbols {
        first.extend_from_slice(&count(offsets[member]).to_be_bytes());
    }
    for &(name, _) in &symbols {
        first.extend_from_slice(name.as_bytes());
        first.push(0);
    }
    let mut sorted = symbols.clone();
    sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut second = Vec::with_capacity(second_size);
    coff::put_u32(&mut second, members.len());
    for &at in &offsets {
        coff::put_u32(&mut second, at);
    }
    coff::put_u32(&mut second, sorted.len());
    for &(_, member) in &sorted {
        coff::put_u16(&mut second, member + 1);
    }
    for &(name, _) in &sorted {
        second.extend_from_slice(name.as_bytes());
        second.push(0);
    }

    // The symbol tables' sizes count their padding, which is zeros.
    first.resize(padded(first_size), 0);
    second.resize(padded(second_size), 0);
    let mut out = b"!<arch>\n".to_vec();
    append(&mut out, "/", "0", &first);
    append(&mut out, "/", "0", &second);
    if !long_names.is_empty() {
        append_long_names(&mut out, &long_names);
    }
    for (member, name) in members.iter().zip(&names) {
        append(&mut out, name, "644", &member.data);
    }
    out
}

/// The offset of `name` in the long-name table `table`, if there.
fn find(table: &[u8], name: &str) -> Option<usize> {
    let mut at = 0;
    for entry in table.split(|&byte| byte == 0) {
        if entry == name.as_bytes() {
            return Some(at);
        }
        at += entry.len() + 1;
    }
    None
}

/// `size` rounded up to an even number, as members are aligned.
fn padded(size: usize) -> usize {
    size + size % 2
}

/// `value` as a 32-bit field of a symbol table.
fn count(value: usize) -> u32 {
    u32::try_from(value).expect("archive offset fits in 32 bits")
}

/// Appends a member: its header (with the given name and mode, the other
/// fields zero), its data and the padding to an even offset.
fn append(out: &mut Vec<u8>, name: &str, mode: &str, data: &[u8]) {
    let header = format!(
        "{name:<16}{:<12}{:<6}{:<6}{mode:<8}{:<10}`\n",
        0,
        0,
        0,
        data.len()
    );
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(b'\n');
    }
}

/// Appends the long-name table, whose header leaves the date, owner and
/// mode fields blank and whose size counts its padding.
fn append_long_names(out: &mut Vec<u8>, table: &[u8]) {
    let size = padded(table.len());
    out.extend_from_slice(format!("{:<48}{size:<10}`\n", "//").as_bytes());
    out.extend_from_slice(table);
    out.resize(out.len() + size - table.len(), b'\n');
}
