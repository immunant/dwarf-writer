use gimli::write::UnitEntryId;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CanonicalTypeName(TypeName);

// Types may have various representations so `TypeName`s should be converted to
// `CanonicalTypeName`s before being compared.
pub type TypeName = Vec<u8>;

pub type TypeMap = HashMap<CanonicalTypeName, UnitEntryId>;

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
