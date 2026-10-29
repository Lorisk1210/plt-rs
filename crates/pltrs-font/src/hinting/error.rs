//! Why bytecode execution stopped, and which program it was.

use std::fmt;

/// One of the three TrueType programs (FreeType's code ranges).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum BytecodeProgram {
    /// The font program (`fpgm`), run once to define functions.
    Font,
    /// The control value program (`prep`), run for every size.
    ControlValue,
    /// A glyph's instructions.
    Glyph,
}

impl fmt::Display for BytecodeProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Font => "font program (fpgm)",
            Self::ControlValue => "control value program (prep)",
            Self::Glyph => "glyph program",
        })
    }
}

/// An error that stops TrueType bytecode, named after FreeType's error
/// codes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub enum BytecodeError {
    /// An opcode that is neither defined nor redefined by an `IDEF`.
    InvalidOpcode(u8),
    /// Pushing beyond the stack size, or calling too deeply.
    StackOverflow,
    /// Reading past the end of a program.
    CodeOverflow,
    /// An argument outside its valid range.
    BadArgument,
    /// `DIV` by zero.
    DivideByZero,
    /// A call to an undefined function.
    InvalidReference,
    /// The `DEBUG` opcode.
    DebugOpcode,
    /// `ENDF` outside a function.
    EndfOutsideFunction,
    /// `FDEF` or `IDEF` inside a definition.
    NestedDefinitions,
    /// A jump into a code range that is not loaded.
    InvalidCodeRange,
    /// Too many instructions, loop calls or backward jumps (an infinite
    /// loop, most likely).
    ExecutionTooLong,
    /// More function definitions than `maxp` allows.
    TooManyFunctionDefs,
    /// More instruction definitions than `maxp` allows.
    TooManyInstructionDefs,
    /// `FDEF` or `IDEF` in a glyph program.
    DefinitionInGlyphProgram,
}

impl fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOpcode(opcode) => write!(f, "invalid opcode {opcode:#04x}"),
            Self::StackOverflow => f.write_str("stack overflow"),
            Self::CodeOverflow => f.write_str("code overflow"),
            Self::BadArgument => f.write_str("bad argument"),
            Self::DivideByZero => f.write_str("division by zero"),
            Self::InvalidReference => f.write_str("invalid reference"),
            Self::DebugOpcode => f.write_str("DEBUG opcode"),
            Self::EndfOutsideFunction => f.write_str("ENDF outside a function"),
            Self::NestedDefinitions => f.write_str("nested function definitions"),
            Self::InvalidCodeRange => f.write_str("invalid code range"),
            Self::ExecutionTooLong => f.write_str("execution too long"),
            Self::TooManyFunctionDefs => f.write_str("too many function definitions"),
            Self::TooManyInstructionDefs => f.write_str("too many instruction definitions"),
            Self::DefinitionInGlyphProgram => f.write_str("FDEF or IDEF in a glyph program"),
        }
    }
}

impl std::error::Error for BytecodeError {}
