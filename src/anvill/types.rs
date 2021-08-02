use super::{PrimitiveType, Type};
use crate::types::CanonicalTypeName;
use anyhow::Result;
use serde::de;
use serde::de::{Deserializer, Unexpected, Visitor};
use serde::Deserialize;
use serde_json::json;
use std::fmt;

impl Type {
    /// Convert an anvill type to our canonical type name for it. Note our
    /// choice of canonical type name is arbitrary but we choose one of its
    /// common names to avoid duplicating debug info as much as possible.
    pub fn name(&self) -> CanonicalTypeName {
        let name: &[u8] = match self {
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
            _ => todo!("missing type {:?}", self),
        };
        name.to_vec().into()
    }

    /// Get the size of an anvill type.
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
