use gimli::constants::*;
use gimli::write::UnitEntryId;
use std::collections::HashMap;
use std::fmt::Formatter;

// Types may have various representations so `TypeName`s should be converted to
// `CanonicalTypeName`s before being compared for equality.
pub type TypeName = Vec<u8>;

// Derive an arbitrary PartialOrd and Ord to allow sorting and deduplicating
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalTypeName(TypeName);

pub type TypeMap = HashMap<DwarfType, UnitEntryId>;

impl std::fmt::Debug for CanonicalTypeName {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match std::str::from_utf8(&self.0) {
            Ok(s) => f.write_str(s),
            _ => f.write_str("Non-UTF8 type name"),
        }
    }
}

/// This enum directly maps onto the way type information is encoded as DWARF
/// info. Derive an arbitrary PartialOrd and Ord to allow sorting and
/// deduplicating.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DwarfType {
    Primitive {
        name: CanonicalTypeName,
        size: Option<u64>,
    },
    Pointer(Box<DwarfType>),
    Typedef {
        name: CanonicalTypeName,
        ref_type: Box<DwarfType>,
    },
    Array {
        inner_type: Box<DwarfType>,
        len: Option<u64>,
    },
    Struct(Vec<DwarfType>),
    Function {
        return_type: Box<DwarfType>,
        args: Vec<DwarfType>,
    },
}

impl DwarfType {
    pub fn void() -> Self {
        DwarfType::Primitive {
            name: b"void".to_vec().into(),
            size: Some(0),
        }
    }

    /// Creates a new primitive type from a canonical type name.
    pub fn new_primitive(name: CanonicalTypeName, size: Option<u64>) -> Self {
        let size = size.or(name.size());
        DwarfType::Primitive { name, size }
    }

    pub fn new_pointer(pointee: DwarfType) -> Self {
        DwarfType::Pointer(Box::new(pointee))
    }

    pub fn new_typedef(name: CanonicalTypeName, ref_ty: DwarfType) -> Self {
        DwarfType::Typedef {
            name,
            ref_type: Box::new(ref_ty),
        }
    }

    pub fn new_array(inner_type: DwarfType, len: Option<u64>) -> Self {
        DwarfType::Array {
            inner_type: Box::new(inner_type),
            len,
        }
    }

    pub fn new_struct(fields: Vec<DwarfType>) -> Self {
        DwarfType::Struct(fields)
    }

    pub fn new_function(return_type: DwarfType, args: Vec<DwarfType>) -> Self {
        DwarfType::Function {
            return_type: Box::new(return_type),
            args,
        }
    }

    pub fn tag(&self) -> DwTag {
        match self {
            DwarfType::Primitive { .. } => DW_TAG_base_type,
            DwarfType::Pointer(_) => DW_TAG_pointer_type,
            DwarfType::Typedef { .. } => DW_TAG_typedef,
            DwarfType::Array { .. } => DW_TAG_array_type,
            DwarfType::Struct(_) => DW_TAG_structure_type,
            // TODO: Double check that subroutine_type is correct
            DwarfType::Function { .. } => DW_TAG_subroutine_type,
        }
    }
}

impl CanonicalTypeName {
    pub fn size(&self) -> Option<u64> {
        match self.0.as_slice() {
            b"bool" | b"_Bool" => Some(1),
            b"int8_t" | b"signed char" | b"i8" => Some(1),
            b"uint8_t" | b"unsigned char" | b"u8" => Some(1),
            b"int16_t" | b"short" | b"i16" => Some(2),
            b"uint16_t" | b"unsigned short" | b"u16" => Some(2),
            b"int32_t" | b"int" | b"i32" => Some(4),
            b"uint32_t" | b"unsigned" | b"u32" => Some(4),
            b"int64_t" | b"long long" | b"i64" => Some(8),
            b"uint64_t" | b"unsigned long long" | b"u64" => Some(8),
            b"int128_t" | b"__int128" | b"i128" => Some(16),
            b"uint128_t" | b"__uint128" | b"u128" => Some(16),
            b"float16_t" | b"binary16" => Some(2),
            b"float" | b"f32" => Some(4),
            b"double" | b"f64" => Some(8),
            b"void" => Some(0),
            _ => None,
        }
    }
}

impl From<TypeName> for CanonicalTypeName {
    fn from(name: TypeName) -> CanonicalTypeName {
        let canonical_name: &[u8] = match name.as_slice() {
            b"bool" | b"_Bool" => b"bool",
            b"int8_t" | b"signed char" | b"i8" => b"int8_t",
            b"uint8_t" | b"unsigned char" | b"u8" => b"uint8_t",
            b"int16_t" | b"short" | b"i16" => b"int16_t",
            b"uint16_t" | b"unsigned short" | b"u16" => b"uint16_t",
            b"int32_t" | b"int" | b"i32" => b"int32_t",
            b"uint32_t" | b"unsigned" | b"u32" => b"uint32_t",
            b"int64_t" | b"long long" | b"i64" => b"int64_t",
            b"uint64_t" | b"unsigned long long" | b"u64" => b"uint64_t",
            b"int128_t" | b"__int128" | b"i128" => b"int128_t",
            b"uint128_t" | b"__uint128" | b"u128" => b"uint128_t",
            b"float16_t" | b"binary16" => b"float16_t",
            b"float" | b"f32" => b"float",
            b"double" | b"f64" => b"double",
            b"long double" => b"long double",
            //M, // uint64_t (x86 MMX vector type)
            b"__float128" => b"__float128",
            b"void" => b"void",
            s => s,
        };
        CanonicalTypeName(canonical_name.to_vec())
    }
}

impl From<CanonicalTypeName> for Vec<u8> {
    fn from(name: CanonicalTypeName) -> Vec<u8> {
        name.0
    }
}
