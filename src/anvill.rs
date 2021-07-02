#![allow(non_camel_case_types)]
use anyhow::{Error, Result};
use serde::de;
use serde::de::{Deserializer, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::Path;
use std::{fmt, fs, io};

impl AnvillHints {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let hints = serde_json::from_reader(reader)?;
        Ok(hints)
    }
}

pub type AnvillFnMap<'a> = HashMap<u64, FunctionRef<'a>>;

#[derive(Debug)]
pub struct FunctionRef<'a> {
    pub func: &'a Function,
    pub name: Option<&'a str>,
}

impl AnvillHints {
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
}

/// Represents a single Anvill input file.
#[derive(Serialize, Deserialize, Debug)]
pub struct AnvillHints {
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
    return_address: Value<Tagged>,
    return_stack_pointer: Option<Value<Untagged>>,
    parameters: Option<Vec<Arg>>,
    return_values: Option<Vec<Value<Tagged>>>,
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
    value: Value<Tagged>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Value<T: ValueLocation> {
    #[serde(flatten)]
    t: T,
    r#type: Type,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Tagged {
    memory { register: Register, offset: u64 },
    register(Register),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum Untagged {
    memory { register: Register, offset: u64 },
    register(Register),
}

pub trait ValueLocation {}
impl ValueLocation for Tagged {}
impl ValueLocation for Untagged {}

#[derive(Deserialize, Serialize, Debug)]
pub struct Memory {
    register: Register,
    offset: u64,
}

// Deriving `PartialOrd` and `Ord` here and for `Type` to allow sorting and
// deduping the Vec of types for a given instance of `AnvillHints`. The ordering
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

impl Type {
    pub fn size(&self) -> u8 {
        match self {
            Type::Bool => 1,
            Type::Primitive(PrimitiveType::b) => 1,
            Type::Primitive(PrimitiveType::B) => 1,
            Type::Primitive(PrimitiveType::h) => 2,
            Type::Primitive(PrimitiveType::H) => 2,
            Type::Primitive(PrimitiveType::i) => 4,
            Type::Primitive(PrimitiveType::I) => 4,
            Type::Primitive(PrimitiveType::l) => 8,
            Type::Primitive(PrimitiveType::L) => 8,
            Type::Primitive(PrimitiveType::o) => 16,
            Type::Primitive(PrimitiveType::O) => 16,
            Type::Primitive(PrimitiveType::e) => 2,
            Type::Primitive(PrimitiveType::f) => 4,
            Type::Primitive(PrimitiveType::d) => 8,
            // TODO: `long double` can be 10 or 12 bytes. How should this be handled?
            Type::Primitive(PrimitiveType::D) => 12,
            //M, // uint64_t (x86 MMX vector type)
            Type::Primitive(PrimitiveType::Q) => 16,
            Type::Primitive(PrimitiveType::v) => 0,
            _ => todo!("missing type"),
        }
    }
}

impl TryFrom<&[u8]> for Type {
    type Error = anyhow::Error;
    fn try_from(s: &[u8]) -> Result<Type> {
        match s {
            b"bool" | b"_Bool" => Ok(Type::Bool),
            b"int8_t" | b"signed char" | b"i8" => Ok(Type::Primitive(PrimitiveType::b)),
            b"uint8_t" | b"unsigned char" | b"u8" => Ok(Type::Primitive(PrimitiveType::B)),
            b"int16_t" | b"short" | b"i16" => Ok(Type::Primitive(PrimitiveType::h)),
            b"uint16_t" | b"unsigned short" | b"u16" => Ok(Type::Primitive(PrimitiveType::H)),
            b"int32_t" | b"int" | b"i32" => Ok(Type::Primitive(PrimitiveType::i)),
            b"uint32_t" | b"unsigned" | b"u32" => Ok(Type::Primitive(PrimitiveType::I)),
            b"int64_t" | b"long long" | b"i64" => Ok(Type::Primitive(PrimitiveType::l)),
            b"uint64_t" | b"unsigned long long" | b"u64" => Ok(Type::Primitive(PrimitiveType::L)),
            b"int128_t" | b"__int128" | b"i128" => Ok(Type::Primitive(PrimitiveType::o)),
            b"uint128_t" | b"__uint128" | b"u128" => Ok(Type::Primitive(PrimitiveType::O)),
            b"float16_t" | b"binary16" => Ok(Type::Primitive(PrimitiveType::e)),
            b"float" | b"f32" => Ok(Type::Primitive(PrimitiveType::f)),
            b"double" | b"f64" => Ok(Type::Primitive(PrimitiveType::d)),
            b"long double" => Ok(Type::Primitive(PrimitiveType::D)),
            //M, // uint64_t (x86 MMX vector type)
            b"__float128" => Ok(Type::Primitive(PrimitiveType::Q)),
            b"void" => Ok(Type::Primitive(PrimitiveType::v)),
            _ => Err(Error::msg("Unknown type")),
        }
    }
}

impl From<&Type> for &'static [u8] {
    fn from(ty: &Type) -> &'static [u8] {
        match ty {
            Type::Bool => b"bool",
            Type::Primitive(PrimitiveType::b) => b"int8_t",
            Type::Primitive(PrimitiveType::B) => b"uint8_t",
            Type::Primitive(PrimitiveType::h) => b"int16_t",
            Type::Primitive(PrimitiveType::H) => b"uint16_t",
            Type::Primitive(PrimitiveType::i) => b"int32_t",
            Type::Primitive(PrimitiveType::I) => b"uint32_t",
            Type::Primitive(PrimitiveType::l) => b"int64_t",
            Type::Primitive(PrimitiveType::L) => b"uint64_t",
            Type::Primitive(PrimitiveType::o) => b"int128_t",
            Type::Primitive(PrimitiveType::O) => b"uint128_t",
            Type::Primitive(PrimitiveType::e) => b"float16_t",
            Type::Primitive(PrimitiveType::f) => b"float",
            Type::Primitive(PrimitiveType::d) => b"double",
            Type::Primitive(PrimitiveType::D) => b"long double",
            //M, // uint64_t (x86 MMX vector type)
            Type::Primitive(PrimitiveType::Q) => b"__float128",
            Type::Primitive(PrimitiveType::v) => b"void",
            _ => todo!("missing type"),
        }
    }
}

struct TypeVisitor;
impl TypeVisitor {
    fn parse_primitive<E: de::Error>(&self, s: &str) -> Result<PrimitiveType, E> {
        serde_json::from_value(json!(s))
            .map_err(|_| de::Error::invalid_value(Unexpected::Str(s), self))
    }

    fn parse_array<E: de::Error>(&self, s: &str) -> Result<(Box<Type>, u64), E> {
        let inner_str = &s[1..s.len() - 1];
        let (inner_str, len) = inner_str
            .rsplit_once("x")
            .expect("Array type did not specify a length");
        let inner_type = Box::new(self.parse_type(inner_str)?);
        let len = len
            .parse()
            .map_err(|_| de::Error::invalid_value(Unexpected::Str(inner_str), self))?;
        Ok((inner_type, len))
    }

    fn parse_type<E: de::Error>(&self, s: &str) -> Result<Type, E> {
        fn is_bracketed(x: &str, left: &str, right: &str) -> bool {
            x.starts_with(left) && x.ends_with(right)
        }
        if s == "?" {
            Ok(Type::Bool)
        } else {
            if s.len() == 1 {
                let ty = self.parse_primitive(s)?;
                Ok(Type::Primitive(ty))
            } else {
                if is_bracketed(s, "[", "]") {
                    let (inner_type, len) = self.parse_array(s)?;
                    Ok(Type::Array { inner_type, len })
                } else if is_bracketed(s, "<", ">") {
                    let (inner_type, len) = self.parse_array(s)?;
                    Ok(Type::Vector { inner_type, len })
                } else if is_bracketed(s, "{", "}") {
                    Ok(Type::Struct)
                } else if is_bracketed(s, "(", ")") {
                    Ok(Type::Function)
                } else if s.starts_with("*") {
                    let indirection_levels = s.chars().take_while(|&c| c == '*').count() as usize;
                    let referent_str = &s[indirection_levels..];
                    let referent_ty = Box::new(self.parse_type(referent_str)?);
                    Ok(Type::Pointer {
                        referent_ty,
                        indirection_levels,
                    })
                } else {
                    Err(de::Error::invalid_value(Unexpected::Str(s), self))
                }
            }
        }
    }
}
impl<'de> Visitor<'de> for TypeVisitor {
    type Value = Type;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an Anvill type")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where E: de::Error {
        self.parse_type(s)
    }
}

impl<'de> Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_str(TypeVisitor)
    }
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

#[derive(Deserialize, Serialize, Debug)]
pub enum X86Register {
    RAX,
    RCX,
    RDX,
    RBX,
    RSI,
    RDI,
    RSP,
    RBP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ARMRegister {
    LR,
    SP,
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
}

#[derive(Deserialize, Serialize, Debug)]
pub enum SPARCRegister {}

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
            let _: AnvillHints =
                serde_json::from_reader(reader).expect(&format!("Failed test {}", test_name));
        }
    }
}
