use super::{PrimitiveType, Type};
use crate::types::{CanonicalTypeName, DwarfType};
use anyhow::Result;
use log::debug;
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
            _ => panic!("Unexpected type {:?}", self),
        };
        name.to_vec().into()
    }

    /// Get the size of an anvill type.
    pub fn size(&self) -> u64 {
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
            _ => todo!("missing type {:?}", self),
        }
    }
}

// This maps the way anvill represents types into the way DWARF encodes types.
impl From<&Type> for DwarfType {
    fn from(anvill_ty: &Type) -> DwarfType {
        match anvill_ty {
            Type::Bool | Type::Primitive(_) => DwarfType::Primitive {
                name: anvill_ty.name(),
                size: Some(anvill_ty.size()),
            },
            Type::Pointer(referent_ty) => DwarfType::Pointer(Box::new(referent_ty.as_ref().into())),
            Type::Array { inner_type, len } => DwarfType::Array {
                inner_type: Box::new(inner_type.as_ref().into()),
                len: Some(*len),
            },
            Type::Struct => {
                debug!("Writing struct info provided by anvill is not supported yet");
                DwarfType::Struct(Vec::new())
            },
            Type::Function => {
                debug!("Writing function type info provided by anvill is not supported yet");
                DwarfType::Function {
                    return_type: Box::new(DwarfType::void()),
                    args: Vec::new(),
                }
            },
            _ => todo!(
                "Map missing type {:?} from an `anvill::Type` type to a `DwarfType`",
                anvill_ty
            ),
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
        } else if s.len() == 1 {
            let ty = self.parse_primitive(s)?;
            Ok(Type::Primitive(ty))
        } else if is_bracketed(s, "[", "]") {
            let (inner_type, len) = self.parse_array(s)?;
            Ok(Type::Array { inner_type, len })
        } else if is_bracketed(s, "<", ">") {
            let (inner_type, len) = self.parse_array(s)?;
            Ok(Type::Vector { inner_type, len })
        } else if is_bracketed(s, "{", "}") {
            Ok(Type::Struct)
        } else if is_bracketed(s, "(", ")") {
            Ok(Type::Function)
        } else if let Some(referent_str) = s.strip_prefix('*') {
            let referent_ty = Box::new(self.parse_type(referent_str)?);
            Ok(Type::Pointer(referent_ty))
        } else if s.starts_with('=') && is_bracketed(&s[2..], "{", "}") {
            debug!("Anvill's identified structs aren't supported yet. {:?} will be treated as normal struct", &s[2..]);
            Ok(Type::Struct)
        } else {
            Err(de::Error::invalid_value(Unexpected::Str(s), self))
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
