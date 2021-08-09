use gimli::constants::*;
use gimli::write::UnitEntryId;
use std::collections::HashMap;

// Types may have various representations so `TypeName`s should be converted to
// `CanonicalTypeName`s before being compared for equality.
pub type TypeName = Vec<u8>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CanonicalTypeName(TypeName);

// This enum directly maps onto the way type information is encoded as DWARF
// info.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DwarfType {
    Primitive {
        name: CanonicalTypeName,
        size: Option<u8>,
    },
    Pointer {
        referent_ty: Box<DwarfType>,
        indirection_levels: usize,
    },
    Array {
        inner_type: Box<DwarfType>,
        len: u64,
    },
    Struct,
    Function,
}

impl DwarfType {
    /// Creates a new primitive type from a canonical type name.
    pub fn new(name: CanonicalTypeName) -> Self {
        DwarfType::Primitive { name, size: None }
    }
    pub fn tag(&self) -> DwTag {
            match self {
                DwarfType::Primitive { .. } => DW_TAG_base_type,
                DwarfType::Pointer { .. } => DW_TAG_pointer_type,
                DwarfType::Array { .. } => DW_TAG_array_type,
                DwarfType::Struct => DW_TAG_structure_type,
                // TODO: Double check that subroutine_type is correct
                DwarfType::Function => DW_TAG_subroutine_type,
            }
    }
}

pub type TypeMap = HashMap<DwarfType, UnitEntryId>;

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
