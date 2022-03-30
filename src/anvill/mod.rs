#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]
use crate::Opt;
use crate::types::DwarfType;
use crate::InputFile;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;

mod types;

impl InputFile for AnvillInput {}

impl AnvillInput {
    /// Anvill data in a format suitable for writing as DWARF debug info.
    pub fn data(&self, cfg: &Opt) -> AnvillData {
        let var_map = if cfg.omit_variables {
            HashMap::new()
        } else {
            self.variables()
        };
        let fn_map = if cfg.omit_functions {
            HashMap::new()
        } else {
            self.functions()
        };
        AnvillData {
            fn_map,
            var_map,
            types: self.types().iter().map(|&t| t.into()).collect(),
        }
    }
}

pub type AnvillFnMap<'a> = HashMap<u64, FunctionRef<'a>>;
pub type AnvillVarMap<'a> = HashMap<u64, VarRef<'a>>;

pub struct AnvillData<'a> {
    pub fn_map: AnvillFnMap<'a>,
    pub var_map: AnvillVarMap<'a>,
    pub types: Vec<DwarfType>,
}

#[derive(Debug)]
pub struct FunctionRef<'a> {
    pub func: &'a Function,
    pub name: Option<&'a str>,
}

pub struct VarRef<'a> {
    pub var: &'a Variable,
    pub name: Option<&'a str>,
}

impl AnvillInput {
    /// Returns a map from addresses to functions, adding its name if it's
    /// provided.
    fn functions(&self) -> AnvillFnMap {
        let mut res = HashMap::new();
        let funcs = self.functions.as_ref();
        let syms = self.symbols.as_ref();
        if let (Some(funcs), Some(syms)) = (funcs, syms) {
            for func in funcs {
                let name = syms
                    .iter()
                    .find(|&sym| sym.address == func.address)
                    .map(|s| s.name.as_str());
                res.insert(func.address, FunctionRef { func, name });
            }
        }
        res
    }

    fn variables(&self) -> AnvillVarMap {
        let mut res = HashMap::new();
        let vars = self.variables.as_ref();
        let syms = self.symbols.as_ref();
        if let (Some(vars), Some(syms)) = (vars, syms) {
            for var in vars {
                let name = syms
                    .iter()
                    .find(|&sym| sym.address == var.address)
                    .map(|s| s.name.as_str());
                res.insert(var.address, VarRef { var, name });
            }
        }
        res
    }

    /// Gets all unique types from variables, function parameters and return
    /// types.
    fn types(&self) -> Vec<&Type> {
        let mut res: Vec<_> = self
            .functions()
            .values()
            .map(|f| f.func.types())
            .flatten()
            .collect();
        if let Some(vars) = &self.variables {
            for var in vars {
                res.push(&var.r#type);
            }
        }
        res.sort();
        res.dedup();
        res
    }
}

impl Function {
    pub fn types(&self) -> Vec<&Type> {
        let mut res = Vec::new();
        if let Some(ret_val) = &self.return_address {
            res.push(&ret_val.r#type);
        }
        if let Some(ret_sp) = &self.return_stack_pointer {
            res.push(&ret_sp.r#type);
        }
        if let Some(params) = &self.parameters {
            for param in params {
                res.push(&param.value.r#type);
            }
        }
        if let Some(ret_values) = &self.return_values {
            for ret_val in ret_values {
                res.push(&ret_val.r#type);
            }
        }
        res
    }
}

impl Arg {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn location(&self) -> Option<&TaggedLocation> {
        self.value.location.as_ref()
    }

    pub fn ty(&self) -> &Type {
        &self.value.r#type
    }
}

/// Represents a single Anvill input file.
#[derive(Serialize, Deserialize, Debug)]
pub struct AnvillInput {
    arch: Arch,
    os: OS,
    functions: Option<Vec<Function>>,
    variables: Option<Vec<Variable>>,
    symbols: Option<Vec<Symbol>>,
    memory: Option<Vec<MemoryRange>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Arch {
    aarch64,
    aarch32,
    x86,
    x86_avx,
    x86_avx512,
    amd64,
    amd64_avx,
    amd64_avx512,
    sparc32,
    sparc64,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum OS {
    linux,
    macos,
    windows,
    solaris,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Function {
    address: u64,
    pub return_address: Option<Value<TaggedLocation>>,
    return_stack_pointer: Option<Value<UntaggedLocation>>,
    pub parameters: Option<Vec<Arg>>,
    pub return_values: Option<Vec<Value<TaggedLocation>>>,
    is_variadic: Option<bool>,
    pub is_noreturn: Option<bool>,
    calling_convention: Option<CallingConvention>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Variable {
    pub r#type: Type,
    pub address: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Symbol {
    address: u64,
    name: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MemoryRange {
    address: u64,
    is_writeable: bool,
    is_executable: bool,
    data: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Arg {
    name: Option<String>,
    #[serde(flatten)]
    value: Value<TaggedLocation>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Value<T: ValueLocation> {
    #[serde(flatten)]
    pub location: Option<T>,
    pub r#type: Type,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum TaggedLocation {
    memory { register: Register, offset: i64 },
    register(Register),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum UntaggedLocation {
    memory { register: Register, offset: i64 },
    register(Register),
}

pub trait ValueLocation {}
impl ValueLocation for TaggedLocation {}
impl ValueLocation for UntaggedLocation {}

// Deriving `PartialOrd` and `Ord` here and for `Type` to allow sorting and
// deduping the Vec of types for a given instance of `AnvillInput`. The ordering
// itself can be completely arbitrary.
#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum PrimitiveType {
    b, // int8_t or signed char
    B, // uint8_t or unsigned char
    h, // int16_t or short
    H, // uint16_t or unsigned short
    i, // int32_t or int
    I, // uint32_t or unsigned
    l, // int64_t or long long
    L, // uint64_t or unsigned long long
    o, // int128_t or __int128
    O, // uint128_t or __uint128
    e, // float16_t or binary16
    f, // float
    d, // double
    D, // long double
    M, // uint64_t (x86 MMX vector type)
    Q, // __float128
    v, // void
}

// This is separate from crate::types::Type to simplify deserializing the anvill
// JSON input.
#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Type {
    Bool, // _Bool or bool
    Primitive(PrimitiveType),
    Pointer(Box<Type>),
    Array { inner_type: Box<Type>, len: u64 },
    Vector { inner_type: Box<Type>, len: u64 },
    Struct,
    Function,
}

#[derive(Deserialize_repr, Serialize_repr, Debug)]
#[repr(u16)]
pub enum CallingConvention {
    C = 0,
    stdcall = 64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum Register {
    X86(X86Register),
    ARM(ARMRegister),
    SPARC(SPARCRegister),
}

// TODO: Add support for x86 registers (i.e. eax, ecx, etc.). Does anvill
// display them as eax or rax?
/// X86 registers
///
/// These variant names directly correspond to the way that anvill represents
/// them in the disassembly JSON output.
#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum X86Register {
    RAX,
    RDX,
    RCX,
    RBX,
    RSI,
    RDI,
    RBP,
    RSP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,

    ST0,
    ST1,
    ST2,
    ST3,
    ST4,
    ST5,
    ST6,
    ST7,

    MM0,
    MM1,
    MM2,
    MM3,
    MM4,
    MM5,
    MM6,
    MM7,

    XMM0,
    XMM1,
    XMM2,
    XMM3,
    XMM4,
    XMM5,
    XMM6,
    XMM7,

    XMM8,
    XMM9,
    XMM10,
    XMM11,
    XMM12,
    XMM13,
    XMM14,
    XMM15,

    XMM16,
    XMM17,
    XMM18,
    XMM19,
    XMM20,
    XMM21,
    XMM22,
    XMM23,
    XMM24,
    XMM25,
    XMM26,
    XMM27,
    XMM28,
    XMM29,
    XMM30,
    XMM31,
}

/// ARM registers
///
/// These variant names directly correspond to the way that anvill represents
/// them in the disassembly JSON output.
#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum ARMRegister {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    SP, // R13
    LR, // R14
    PC, // R15

    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
    D11,
    D12,
    D13,
    D14,
    D15,
    D16,
    D17,
    D18,
    D19,
    D20,
    D21,
    D22,
    D23,
    D24,
    D25,
    D26,
    D27,
    D28,
    D29,
    D30,
    D31,

    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    S12,
    S13,
    S14,
    S15,
    S16,
    S17,
    S18,
    S19,
    S20,
    S21,
    S22,
    S23,
    S24,
    S25,
    S26,
    S27,
    S28,
    S29,
    S30,
    S31,

    // TODO: Add Q0-Q15. This requires refactoring the IntoGimli impl for
    // anvill::Register since Q0 is D0+D1, etc.
}

// TODO: Fill this in. Set variant values to the DWARF register number since
// gimli's `name_to_register` currently doesn't support SPARC.
/// SPARC registers
#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum SPARCRegister {}

impl From<Register> for u16 {
    fn from(r: Register) -> u16 {
        match r {
            Register::X86(r) => r as u16,
            Register::ARM(r) => r as u16,
            Register::SPARC(r) => r as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;

    const TEST_DIR: &str = "tests/anvill_json";
    fn get_tests() -> impl Iterator<Item = String> {
        let all_files = fs::read_dir(TEST_DIR).expect("Could not open test directory");

        all_files.filter_map(|file| file.ok()).filter_map(|file| {
            let name = file
                .file_name()
                .into_string()
                .expect("Could not convert `OsString` to UTF-8");
            Some(name)
        })
    }

    #[test]
    fn parse_anvill_json() {
        for test_name in get_tests() {
            println!("Running test case: {}", test_name);
            let file = fs::File::open(format!("{}/{}", TEST_DIR, test_name))
                .expect(&format!("Could not open test {}", test_name));
            let reader = io::BufReader::new(file);
            let _: AnvillInput =
                serde_json::from_reader(reader).expect(&format!("Failed test {}", test_name));
        }
    }
}
