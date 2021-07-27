#![allow(non_camel_case_types)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

mod types;

impl AnvillInput {
    /// Loads a file to create a new `AnvillInput`.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let hints = serde_json::from_reader(reader)?;
        Ok(hints)
    }

    /// Anvill data in a format suitable for writing as DWARF debug info.
    pub fn data(&self) -> AnvillData {
        AnvillData {
            fn_map: self.functions(),
            types: self.types(),
        }
    }
}

pub type AnvillFnMap<'a> = HashMap<u64, FunctionRef<'a>>;

pub struct AnvillData<'a> {
    pub fn_map: AnvillFnMap<'a>,
    pub types: Vec<&'a Type>,
}

#[derive(Debug)]
pub struct FunctionRef<'a> {
    pub func: &'a Function,
    pub name: Option<&'a str>,
}

impl AnvillInput {
    /// Returns a map from addresses to functions, adding its name if it's
    /// provided.
    pub fn functions(&self) -> AnvillFnMap {
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

    /// Gets all unique types from variables, function parameters and return
    /// types.
    pub fn types(&self) -> Vec<&Type> {
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
    pub fn parameters(&self) -> Option<&Vec<Arg>> {
        self.parameters.as_ref()
    }

    pub fn types(&self) -> Vec<&Type> {
        let mut res = vec![&self.return_address.r#type];
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
        self.name.as_ref().map(|s| s.as_str())
    }

    pub fn location(&self) -> &TaggedLocation {
        &self.value.location
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
    return_address: Value<TaggedLocation>,
    return_stack_pointer: Option<Value<UntaggedLocation>>,
    parameters: Option<Vec<Arg>>,
    return_values: Option<Vec<Value<TaggedLocation>>>,
    is_variadic: Option<bool>,
    is_noreturn: Option<bool>,
    calling_convention: Option<CallingConvention>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Variable {
    r#type: Type,
    address: u64,
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
    location: T,
    r#type: Type,
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

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Type {
    Bool, // _Bool or bool
    Primitive(PrimitiveType),
    Pointer {
        referent_ty: Box<Type>,
        indirection_levels: usize,
    },
    Array {
        inner_type: Box<Type>,
        len: u64,
    },
    Vector {
        inner_type: Box<Type>,
        len: u64,
    },
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

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum X86Register {
    RAX = 0,
    RDX = 1,
    RCX = 2,
    RBX = 3,
    RSI = 4,
    RDI = 5,
    RBP = 6,
    RSP = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum ARMRegister {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    SP = 13,
    LR = 14,
    PC = 15,
}

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

    const TEST_DIR: &str = "anvill-tests/json";
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
    fn pate_tests() {
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
