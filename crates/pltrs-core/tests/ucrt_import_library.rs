//! The import library of the UCRT's maths functions that the crate's build
//! script writes on Windows with a GNU toolchain (`build/`): byte for byte
//! what LLVM's `lib` tool writes for the same functions, and the member
//! defining `sincos` and `sincosf`, indexed and calling the UCRT's `sin`
//! and `cos`.

#[path = "../build/coff.rs"]
mod coff;
#[path = "../build/import_library.rs"]
mod import_library;
#[path = "../build/sincos.rs"]
mod sincos;

/// The UCRT's maths API set.
const DLL: &str = "api-ms-win-crt-math-l1-1-0.dll";

/// What `rust-lld -flavor link /lib /def:ucrt.def /machine:x64` (LLVM
/// 23's `lib` tool) writes for a `.def` file naming `sin`, `cos` and `pow`
/// of [`DLL`].
const LLVM_LIBRARY: &[u8] = include_bytes!("fixtures/ucrt-math-sin-cos-pow.lib");

/// The 32-bit big-endian field at `at`.
fn be32(bytes: &[u8], at: usize) -> usize {
    u32::from_be_bytes(bytes[at..at + 4].try_into().expect("4 bytes")) as usize
}

/// The 32-bit little-endian field at `at`.
fn le32(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes")) as usize
}

/// The archive's first symbol table: each symbol with the offset of the
/// header of the member defining it.
fn symbol_table(archive: &[u8]) -> Vec<(String, usize)> {
    const DATA: usize = 8 + 60;
    let count = be32(archive, DATA);
    let names = archive[DATA + 4 + 4 * count..]
        .split(|&byte| byte == 0)
        .map(|name| String::from_utf8(name.to_vec()).expect("ASCII names"));
    names
        .zip((0..count).map(|index| be32(archive, DATA + 4 + 4 * index)))
        .collect()
}

#[test]
fn import_library_is_what_llvm_writes() {
    let library = import_library::write(DLL, &["sin", "cos", "pow"], Vec::new());
    assert_eq!(library, LLVM_LIBRARY);
}

#[test]
fn sincos_is_indexed_and_calls_the_ucrt() {
    let library =
        import_library::write(DLL, &["sin", "cos", "sinf", "cosf"], vec![sincos::member()]);
    let symbols = symbol_table(&library);
    let member = |name: &str| symbols.iter().find(|(symbol, _)| symbol == name).unwrap().1;
    let at = member("sincos");
    assert_eq!(member("sincosf"), at);
    assert_eq!(&library[at..at + 16], b"sincos.o/       ");
    for function in ["sin", "cos", "sinf", "cosf"] {
        assert_ne!(member(function), at);
        assert_eq!(member(&format!("__imp_{function}")), member(function));
    }

    // The object: x86-64, three sections, and its undefined symbols the
    // import address table entries of the UCRT's functions.
    let object = &library[at + 60..];
    assert_eq!(&object[..4], &[0x64, 0x86, 3, 0]);
    let symbols_at = le32(object, 8);
    let count = le32(object, 12);
    let strings = &object[symbols_at + 18 * count..];
    let undefined: Vec<String> = (0..count)
        .map(|index| &object[symbols_at + 18 * index..][..18])
        .filter(|symbol| symbol[12..14] == [0, 0])
        .map(|symbol| {
            let name = if symbol[..4] == [0; 4] {
                let start = le32(symbol, 4);
                strings[start..].split(|&byte| byte == 0).next().unwrap()
            } else {
                symbol[..8].split(|&byte| byte == 0).next().unwrap()
            };
            String::from_utf8(name.to_vec()).unwrap()
        })
        .collect();
    assert_eq!(
        undefined,
        ["__imp_sin", "__imp_cos", "__imp_sinf", "__imp_cosf"]
    );
}
