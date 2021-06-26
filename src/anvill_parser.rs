#![allow(non_camel_case_types)]
use crate::Hints;
use serde::de;
use serde::de::{DeserializeOwned, Deserializer, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt;

impl<A: Arch> Hints for AnvillHints<A> {
    fn fn_names(&self) -> Option<Vec<&String>> {
        let fns = self.functions.as_ref();
        let syms = self.symbols.as_ref();
        if let (Some(func_vec), Some(sym_vec)) = (fns, syms) {
            let addr_matches = func_vec.iter().filter_map(|func| {
                sym_vec
                    .iter()
                    .find(|&sym| sym.address == func.address)
                    .map(|s| &s.name)
            });
            Some(addr_matches.collect())
        } else {
            None
        }
    }
}

/// Represents a single Anvill input file.
#[derive(Serialize, Deserialize, Debug)]
pub struct AnvillHints<A: Arch> {
    arch: A,
    os: OS,
    functions: Option<Vec<Function<A>>>,
    variables: Option<Vec<Variable>>,
    symbols: Option<Vec<Symbol>>,
    memory: Option<Vec<MemoryRange>>,
}

/// The characteristics of an architecture supported by Anvill.
pub trait Arch: fmt::Debug {
    type Register: DeserializeOwned + Serialize + fmt::Debug;
    type CallingConvention: DeserializeOwned + Serialize + fmt::Debug;
}

#[derive(Deserialize, Serialize, Debug)]
pub enum OS {
    linux,
    macos,
    windows,
    solaris,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Function<A: Arch> {
    address: u64,
    return_address: Value<Tagged<A>>,
    return_stack_pointer: Option<Value<Untagged<A>>>,
    parameters: Option<Vec<Arg<A>>>,
    return_values: Vec<Value<Tagged<A>>>,
    is_variadic: Option<bool>,
    is_noreturn: Option<bool>,
    calling_convention: Option<A::CallingConvention>,
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
pub struct Arg<A: Arch> {
    name: Option<String>,
    #[serde(flatten)]
    value: Value<Tagged<A>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Value<T: ValueLocation> {
    #[serde(flatten)]
    t: T,
    r#type: Type,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Tagged<A: Arch> {
    memory { register: A::Register, offset: u64 },
    register(A::Register),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum Untagged<A: Arch> {
    memory { register: A::Register, offset: u64 },
    register(A::Register),
}

pub trait ValueLocation {}
impl<A: Arch> ValueLocation for Tagged<A> {}
impl<A: Arch> ValueLocation for Untagged<A> {}

#[derive(Deserialize, Serialize, Debug)]
pub struct Memory<A: Arch> {
    register: A::Register,
    offset: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum PrimitiveType {
    b, // i8
    B, // u8
    h, // i16
    H, // u16
    i, // i32
    I, // u32
    l, // i64
    L, // u64
    o, // i128
    O, // u128
    e, // f16
    f, // f32
    d, // f64
    D, // long double
    M, // mmx
    Q, // f128
    v, // void
}
#[derive(Serialize, Debug)]
pub enum Type {
    Bool,
    Primitive(PrimitiveType),
    Pointer {
        r#type: PrimitiveType,
        indirection_levels: u64,
    },
}

struct TypeVisitor;
impl<'de> Visitor<'de> for TypeVisitor {
    type Value = Type;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an Anvill type")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where E: de::Error {
        match s {
            "?" => Ok(Type::Bool),
            _ if s.len() == 1 => {
                let ty = serde_json::from_value(json!(s))
                    .map_err(|_| de::Error::invalid_value(Unexpected::Str(s), &self))?;
                Ok(Type::Primitive(ty))
            },
            _ => {
                let indirection_levels = s.chars().take_while(|&c| c == '*').count() as u64;
                let referent_ty = s
                    .chars()
                    .find(|&c| c != '*')
                    .ok_or(de::Error::invalid_value(Unexpected::Str(s), &self))?;
                let r#type = serde_json::from_value(json!(referent_ty))
                    .map_err(|_| de::Error::invalid_value(Unexpected::Str(s), &self))?;
                Ok(Type::Pointer {
                    r#type,
                    indirection_levels,
                })
            },
        }
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
pub enum X86 {
    x86,
    x86_avx,
    x86_avx512,
    amd64,
    amd64_avx,
    amd64_avx512,
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

impl Arch for X86 {
    type Register = X86Register;
    type CallingConvention = CallingConvention;
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ARM {
    aarch64,
    aarch32,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ARMRegister {
    Lr,
}

impl Arch for ARM {
    type Register = ARMRegister;
    type CallingConvention = CallingConvention;
}

#[derive(Deserialize, Serialize, Debug)]
pub enum SPARC {
    sparc32,
    sparc64,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum SPARCRegister {}

impl Arch for SPARC {
    type Register = SPARCRegister;
    type CallingConvention = CallingConvention;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;

    fn get_tests() -> impl Iterator<Item = String> {
        let all_files = fs::read_dir("anvill-tests").expect("Could not open test directory");

        all_files.filter_map(|file| file.ok()).filter_map(|file| {
            let name = file
                .file_name()
                .into_string()
                .expect("Could not convert `OsString` to UTF-8");
            let is_test = name.ends_with("anvill.json");
            if is_test {
                Some(name)
            } else {
                None
            }
        })
    }

    #[test]
    fn pate_tests() {
        for test_name in get_tests() {
            println!("Running test case: {}", test_name);
            let file = fs::File::open(format!("anvill-tests/{}", test_name))
                .expect(&format!("Could not open test {}", test_name));
            let reader = io::BufReader::new(file);
            let _: AnvillHints<X86> =
                serde_json::from_reader(reader).expect(&format!("Failed test {}", test_name));
        }
    }
}
