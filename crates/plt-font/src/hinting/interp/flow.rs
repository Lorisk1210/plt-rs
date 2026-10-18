//! Control flow: conditionals, jumps, function and instruction
//! definitions and calls.

use super::{Call, Exec, MAX_CALL_DEPTH, opcode_length, range_index};
use crate::hinting::error::{BytecodeError, BytecodeProgram};

const IF: u8 = 0x58;
const ELSE: u8 = 0x1B;
const EIF: u8 = 0x59;
const FDEF: u8 = 0x2C;
const ENDF: u8 = 0x2D;
const IDEF: u8 = 0x89;

/// A function or instruction definition (`TT_DefRecord`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Definition {
    range: BytecodeProgram,
    start: i64,
    end: i64,
    /// Function number or opcode.
    number: u32,
    active: bool,
}

impl Definition {
    /// A record taken but never filled in (FreeType counts it anyway).
    const EMPTY: Self = Self {
        range: BytecodeProgram::Font,
        start: 0,
        end: 0,
        number: 0,
        active: false,
    };
}

/// The definitions the font and control value programs make, with the
/// capacities `maxp` declares.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct DefinitionTable {
    functions: Vec<Definition>,
    max_functions: usize,
    /// Highest function number defined (`maxFunc`).
    max_function: u32,
    instructions: Vec<Definition>,
    max_instructions: usize,
}

impl DefinitionTable {
    /// An empty table with room for the given numbers of definitions.
    pub(crate) fn new(max_functions: u16, max_instructions: u16) -> Self {
        Self {
            max_functions: usize::from(max_functions),
            max_instructions: usize::from(max_instructions),
            ..Self::default()
        }
    }

    fn list(&mut self, kind: Kind) -> &mut Vec<Definition> {
        match kind {
            Kind::Function => &mut self.functions,
            Kind::Instruction => &mut self.instructions,
        }
    }

    /// The bookkeeping of `FDEF`/`IDEF` `number` at `start` in `range`:
    /// reuses the record of an earlier definition with that number or takes
    /// a new one (counted even when the number then proves invalid).
    /// Returns the record's index.
    fn record(
        &mut self,
        kind: Kind,
        number: i64,
        range: BytecodeProgram,
        start: i64,
    ) -> Result<usize, BytecodeError> {
        let (max, limit, too_many) = match kind {
            Kind::Function => (
                self.max_functions,
                0xFFFF,
                BytecodeError::TooManyFunctionDefs,
            ),
            Kind::Instruction => (
                self.max_instructions,
                0xFF,
                BytecodeError::TooManyInstructionDefs,
            ),
        };
        let list = self.list(kind);
        let index = match list
            .iter()
            .position(|def| u64::from(def.number) == number as u64)
        {
            Some(index) => index,
            None if list.len() >= max => return Err(too_many),
            None => {
                list.push(Definition::EMPTY);
                list.len() - 1
            }
        };
        if !(0..=limit).contains(&number) {
            return Err(too_many);
        }
        list[index] = Definition {
            range,
            start,
            number: number as u32,
            active: true,
            ..list[index]
        };
        if kind == Kind::Function {
            self.max_function = self.max_function.max(number as u32);
        }
        Ok(index)
    }

    /// The function called `number` (`Ins_CALL`'s lookup: direct when
    /// functions were defined in order, by search otherwise).
    fn function(&self, number: u64) -> Option<Definition> {
        if number > u64::from(self.max_function) {
            return None;
        }
        let direct = self.functions.get(number as usize).filter(|def| {
            self.max_function as usize + 1 == self.functions.len()
                && u64::from(def.number) == number
        });
        direct
            .or_else(|| {
                self.functions
                    .iter()
                    .find(|def| u64::from(def.number) == number)
            })
            .copied()
    }
}

/// The definitions a program may use: writable in the font and control
/// value programs, read-only in glyph programs.
#[derive(Debug)]
pub(crate) enum Definitions<'r> {
    Writable(&'r mut DefinitionTable),
    ReadOnly(&'r DefinitionTable),
}

impl Definitions<'_> {
    fn table(&self) -> &DefinitionTable {
        match self {
            Self::Writable(table) => table,
            Self::ReadOnly(table) => table,
        }
    }

    fn writable(&mut self) -> Option<&mut DefinitionTable> {
        match self {
            Self::Writable(table) => Some(table),
            Self::ReadOnly(_) => None,
        }
    }
}

/// Which list a definition goes to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Function,
    Instruction,
}

impl Exec<'_> {
    /// Advances to the next opcode without executing it (`SkipCode`).
    fn skip_code(&mut self) -> bool {
        self.ip = self.ip.wrapping_add(self.length);
        if let Ok(ip) = usize::try_from(self.ip)
            && let Some(length) = opcode_length(self.code, ip)
            && self.charge(1)
        {
            self.opcode = self.code[ip];
            self.length = length;
            return true;
        }
        if self.error.is_none() {
            self.fail(BytecodeError::CodeOverflow);
        }
        false
    }

    /// `IF`: on false, skips to the matching `ELSE` or `EIF`.
    pub(super) fn ins_if(&mut self) {
        if self.arg(0) != 0 {
            return;
        }
        let mut nesting = 1;
        loop {
            if !self.skip_code() {
                return;
            }
            let out = match self.opcode {
                IF => {
                    nesting += 1;
                    false
                }
                ELSE => nesting == 1,
                EIF => {
                    nesting -= 1;
                    nesting == 0
                }
                _ => false,
            };
            if out {
                return;
            }
        }
    }

    /// `ELSE` reached by executing the true branch: skips to the matching
    /// `EIF`.
    pub(super) fn ins_else(&mut self) {
        let mut nesting = 1;
        while nesting != 0 {
            if !self.skip_code() {
                return;
            }
            match self.opcode {
                IF => nesting += 1,
                EIF => nesting -= 1,
                _ => {}
            }
        }
    }

    /// `JMPR`: relative jump; backward jumps count against the loop budget.
    pub(super) fn ins_jmpr(&mut self) {
        let offset = self.arg(0);
        if offset == 0 && self.args == 0 {
            return self.fail(BytecodeError::BadArgument);
        }
        self.ip = self.ip.wrapping_add(offset);
        let beyond_function = self
            .calls
            .last()
            .is_some_and(|call| self.ip > call.definition.end);
        if self.ip < 0 || beyond_function {
            return self.fail(BytecodeError::BadArgument);
        }
        self.length = 0;
        if offset < 0 {
            self.neg_jump_counter += 1;
            if self.neg_jump_counter > self.loop_limit {
                self.fail(BytecodeError::ExecutionTooLong);
            }
        }
    }

    /// `JROT`: jumps when the condition is true.
    pub(super) fn ins_jrot(&mut self) {
        if self.arg(1) != 0 {
            self.ins_jmpr();
        }
    }

    /// `JROF`: jumps when the condition is false.
    pub(super) fn ins_jrof(&mut self) {
        if self.arg(1) == 0 {
            self.ins_jmpr();
        }
    }

    /// Continues in another code range (`Ins_Goto_CodeRange`).
    fn goto_code_range(&mut self, range: BytecodeProgram, ip: i64) {
        let Some(code) = self.ranges[range_index(range)] else {
            return self.fail(BytecodeError::InvalidCodeRange);
        };
        if ip > code.len() as i64 {
            return self.fail(BytecodeError::CodeOverflow);
        }
        self.code = code;
        self.ip = ip;
        self.length = 0;
        self.range = range;
    }

    /// Calls `definition` `count` times.
    fn enter(&mut self, definition: Definition, count: i64) {
        self.calls.push(Call {
            caller_range: self.range,
            caller_ip: self.ip + 1,
            count,
            definition,
        });
        self.goto_code_range(definition.range, definition.start);
    }

    /// `FDEF` and `IDEF`: records a definition and skips its body.
    fn define(&mut self, kind: Kind) {
        let (number, range, start) = (self.arg(0), self.range, self.ip + 1);
        let recorded = match self.state.definitions.writable() {
            Some(table) => table.record(kind, number, range, start),
            None => Err(BytecodeError::DefinitionInGlyphProgram),
        };
        let index = match recorded {
            Ok(index) => index,
            Err(error) => return self.fail(error),
        };
        while self.skip_code() {
            match self.opcode {
                FDEF | IDEF => return self.fail(BytecodeError::NestedDefinitions),
                ENDF => {
                    let end = self.ip;
                    if let Some(table) = self.state.definitions.writable() {
                        table.list(kind)[index].end = end;
                    }
                    return;
                }
                _ => {}
            }
        }
    }

    /// `FDEF`.
    pub(super) fn ins_fdef(&mut self) {
        self.define(Kind::Function);
    }

    /// `IDEF`.
    pub(super) fn ins_idef(&mut self) {
        self.define(Kind::Instruction);
    }

    /// `ENDF`: loops the function again or returns to the caller.
    pub(super) fn ins_endf(&mut self) {
        let Some(mut call) = self.calls.pop() else {
            return self.fail(BytecodeError::EndfOutsideFunction);
        };
        call.count -= 1;
        if call.count > 0 {
            self.ip = call.definition.start;
            self.length = 0;
            self.calls.push(call);
        } else {
            self.goto_code_range(call.caller_range, call.caller_ip);
        }
    }

    /// The function a `CALL` or `LOOPCALL` argument names, if active.
    fn callable(&mut self, number: i64) -> Option<Definition> {
        let found = self
            .state
            .definitions
            .table()
            .function(number as u64)
            .filter(|def| def.active);
        if found.is_none() {
            self.fail(BytecodeError::InvalidReference);
        }
        found
    }

    /// `CALL`.
    pub(super) fn ins_call(&mut self) {
        let Some(definition) = self.callable(self.arg(0)) else {
            return;
        };
        if self.calls.len() >= MAX_CALL_DEPTH {
            return self.fail(BytecodeError::StackOverflow);
        }
        self.enter(definition, 1);
    }

    /// `LOOPCALL`: calls a function repeatedly, against the loop budget.
    pub(super) fn ins_loopcall(&mut self) {
        let Some(definition) = self.callable(self.arg(1)) else {
            return;
        };
        if self.calls.len() >= MAX_CALL_DEPTH {
            return self.fail(BytecodeError::StackOverflow);
        }
        let count = self.arg(0);
        if count > 0 {
            self.enter(definition, i64::from(count as i32));
            self.loopcall_counter = self.loopcall_counter.saturating_add(count as u64);
            if self.loopcall_counter > self.loop_limit {
                self.fail(BytecodeError::ExecutionTooLong);
            }
        }
    }

    /// An opcode without built-in meaning: runs its `IDEF`, if any.
    pub(super) fn ins_unknown(&mut self) {
        let opcode = self.opcode;
        let found = self
            .state
            .definitions
            .table()
            .instructions
            .iter()
            .find(|def| def.number as u8 == opcode && def.active)
            .copied();
        match found {
            Some(_) if self.calls.len() >= MAX_CALL_DEPTH => {
                self.fail(BytecodeError::StackOverflow);
            }
            Some(definition) => self.enter(definition, 1),
            None => self.fail(BytecodeError::InvalidOpcode(opcode)),
        }
    }
}
