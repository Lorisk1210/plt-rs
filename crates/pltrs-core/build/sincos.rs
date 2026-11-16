//! `sincos` and `sincosf` from the UCRT's `sin` and `cos`.
//!
//! On GNU targets LLVM computes a `sin` and a `cos` of the same value
//! with one call, `sincos(x, &mut s, &mut c)` (`sincosf` for `f32`),
//! which MinGW-w64 implements with the x87 `fsincos` and the UCRT does
//! not have. The object written here defines the two functions for the
//! import library: each saves its arguments, calls `sin` and then `cos`
//! through their entries of the import address table (`__imp_sin`,
//! `__imp_cos`), and stores the results. The code follows the Windows
//! x64 calling convention (the argument in `xmm0`, the two pointers in
//! `rdx` and `r8`, 32 bytes of shadow space for the callee), with the
//! unwind information Windows needs to walk the stack through it.

use crate::coff::{self, Relocation, RelocationKind, Section, StorageClass, Symbol};
use crate::import_library::Member;

/// The two functions: their names, the `sin` and `cos` they call and the
/// prefix of the SSE moves of their floating-point type (`movsd` for
/// `f64`, `movss` for `f32`).
const FUNCTIONS: [(&str, &str, &str, u8); 2] = [
    ("sincos", "sin", "cos", 0xF2),
    ("sincosf", "sinf", "cosf", 0xF3),
];

/// The distance between the two functions in `.text`.
const STRIDE: u32 = 0x40;

/// The machine code of a function whose SSE moves take the prefix
/// `prefix`; the two calls' displacements (at [`CALL_SIN`] and
/// [`CALL_COS`]) are left for the relocations.
fn code(prefix: u8) -> [u8; 51] {
    [
        0x56, // push rsi
        0x57, // push rdi
        0x48, 0x83, 0xEC, 0x38, // sub rsp, 0x38
        0x48, 0x89, 0xD6, // mov rsi, rdx
        0x4C, 0x89, 0xC7, // mov rdi, r8
        prefix, 0x0F, 0x11, 0x44, 0x24, 0x28, // movs[sd] [rsp+0x28], xmm0
        0xFF, 0x15, 0, 0, 0, 0, // call [rip+__imp_sin]
        prefix, 0x0F, 0x11, 0x06, // movs[sd] [rsi], xmm0
        prefix, 0x0F, 0x10, 0x44, 0x24, 0x28, // movs[sd] xmm0, [rsp+0x28]
        0xFF, 0x15, 0, 0, 0, 0, // call [rip+__imp_cos]
        prefix, 0x0F, 0x11, 0x07, // movs[sd] [rdi], xmm0
        0x48, 0x83, 0xC4, 0x38, // add rsp, 0x38
        0x5F, // pop rdi
        0x5E, // pop rsi
        0xC3, // ret
    ]
}

/// The offset of the displacement of the call of `sin` in [`code`].
const CALL_SIN: u32 = 0x14;
/// The offset of the displacement of the call of `cos` in [`code`].
const CALL_COS: u32 = 0x24;

/// The unwind information both functions share (`UNWIND_INFO`): version
/// 1, a prologue of 6 bytes and its three operations, latest first: the
/// allocation of 0x38 bytes (`UWOP_ALLOC_SMALL`) after offset 6, the
/// pushes of `rdi` (7) and `rsi` (6) (`UWOP_PUSH_NONVOL`) after offsets 2
/// and 1; the count of operations is padded to an even number.
const UNWIND_INFO: [u8; 12] = [
    0x01, 0x06, 0x03, 0x00, 0x06, 0x62, 0x02, 0x70, 0x01, 0x60, 0x00, 0x00,
];

/// The symbol indices of the object: the two sections the relocations
/// refer to, the two functions, then each function's `__imp_` pointers.
const TEXT: u32 = 0;
/// The index of the `.xdata` section's symbol.
const XDATA: u32 = 1;
/// The index of the first `__imp_` symbol.
const IMPORTS: u32 = 4;

/// The member of the import library that defines `sincos` and `sincosf`.
pub(crate) fn member() -> Member {
    let mut text = Vec::new();
    let mut text_relocations = Vec::new();
    let mut pdata = Vec::new();
    let mut pdata_relocations = Vec::new();
    let mut symbols = vec![
        Symbol::new(".text", 1, 0, StorageClass::Static),
        Symbol::new(".xdata", 2, 0, StorageClass::Static),
    ];
    let mut imports = Vec::new();
    for (index, &(name, sin, cos, prefix)) in (0u32..).zip(&FUNCTIONS) {
        let start = index * STRIDE;
        let code = code(prefix);
        text.resize(start as usize, 0xCC);
        text.extend_from_slice(&code);
        for (call, symbol) in [(CALL_SIN, sin), (CALL_COS, cos)] {
            text_relocations.push(Relocation {
                offset: start + call,
                symbol: IMPORTS + imports.len() as u32,
                kind: RelocationKind::Relative,
            });
            imports.push(Symbol::undefined(&format!("__imp_{symbol}")));
        }
        // RUNTIME_FUNCTION: the function's start and end, and its unwind
        // information.
        let end = start + code.len() as u32;
        for (symbol, value) in [(TEXT, start), (TEXT, end), (XDATA, 0)] {
            pdata_relocations.push(Relocation {
                offset: pdata.len() as u32,
                symbol,
                kind: RelocationKind::Rva,
            });
            pdata.extend_from_slice(&value.to_le_bytes());
        }
        let mut function = Symbol::new(name, 1, start, StorageClass::External);
        function.function = true;
        symbols.push(function);
    }
    symbols.extend(imports);
    let sections = [
        Section {
            name: ".text",
            characteristics: coff::CODE | coff::align(16),
            data: text,
            relocations: text_relocations,
        },
        Section {
            name: ".xdata",
            characteristics: coff::READ_ONLY_DATA | coff::align(4),
            data: UNWIND_INFO.to_vec(),
            relocations: Vec::new(),
        },
        Section {
            name: ".pdata",
            characteristics: coff::READ_ONLY_DATA | coff::align(4),
            data: pdata,
            relocations: pdata_relocations,
        },
    ];
    Member {
        name: "sincos.o".to_owned(),
        data: coff::object(&sections, &symbols),
        symbols: FUNCTIONS.iter().map(|f| f.0.to_owned()).collect(),
    }
}
