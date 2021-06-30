#![allow(non_camel_case_types)]
use anyhow::Result;
use serde::de;
use serde::de::{DeserializeOwned, Deserializer, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::{fmt, fs, io};

impl AnvillHints<ARM> {
    pub fn new(path: &str) -> Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let hints = serde_json::from_reader(reader)?;
        Ok(hints)
    }
}

pub type AnvillFnMap<'a, A> = HashMap<u64, NamedFunction<'a, A>>;

pub struct NamedFunction<'a, A: Arch> {
    pub func: &'a Function<A>,
    pub name: Option<&'a str>,
}

impl<A: Arch> AnvillHints<A> {
    pub fn functions(&self) -> AnvillFnMap<A> {
        let mut res = HashMap::new();
        let funcs = self.functions.as_ref();
        let syms = self.symbols.as_ref();
        if let (Some(funcs), Some(syms)) = (funcs, syms) {
            for func in funcs {
                let name = syms
                    .iter()
                    .find(|&sym| sym.address == func.address)
                    .map(|s| s.name.as_str());
                res.insert(func.address, NamedFunction { func, name });
            }
        }
        res
    }
}

impl<A: Arch> Function<A> {
    pub fn parameters(&self) -> Option<&Vec<Arg<A>>> {
        self.parameters.as_ref()
    }
}

impl<A: Arch> Arg<A> {
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|s| s.as_str())
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
    return_values: Option<Vec<Value<Tagged<A>>>>,
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
            let _: AnvillHints<X86> =
                serde_json::from_reader(reader).expect(&format!("Failed test {}", test_name));
        }
    }
}
