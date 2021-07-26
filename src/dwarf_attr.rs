use crate::anvill;
use crate::into_gimli::IntoGimli;
use anyhow::Result;
use gimli::write::{Address, AttributeValue, Expression, StringTable};
use std::convert::TryInto;

/// Converts an anvill location to an `AttributeValue`.
pub fn dwarf_location(location: &anvill::TaggedLocation) -> AttributeValue {
    use anvill::TaggedLocation;

    let mut expr = Expression::new();
    match location {
        TaggedLocation::register(reg) => expr.op_reg(reg.into_gimli()),
        TaggedLocation::memory { register, offset } => expr.op_breg(register.into_gimli(), *offset),
    }
    AttributeValue::Exprloc(expr)
}

/// Converts an `AttributeValue` to an anvill type or returns `None` for custom
/// types. Note this may produce false `None`s.
pub fn name_to_anvill_ty(attr: &AttributeValue, strings: &StringTable) -> Option<anvill::Type> {
    let name: Result<anvill::Type> = name_as_bytes(attr, strings).try_into();
    name.ok()
}

pub fn name_as_bytes<'a>(attr: &'a AttributeValue, strings: &'a StringTable) -> &'a [u8] {
    // TODO: This is missing some cases
    match attr {
        AttributeValue::String(s) => s,
        AttributeValue::StringRef(str_id) => strings.get(*str_id),
        _ => panic!("Unhandled `AttributeValue` variant in `name_as_bytes`"),
    }
}

// TODO: This should probably be merged with `high_pc_to_u64`
pub fn low_pc_to_u64(attr: &AttributeValue) -> u64 {
    // TODO: Handle Address::Symbol
    match attr {
        AttributeValue::Address(Address::Constant(addr)) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `low_pc_to_u64`"),
    }
}

pub fn high_pc_to_u64(attr: &AttributeValue) -> u64 {
    match attr {
        AttributeValue::Address(Address::Constant(addr)) => *addr,
        AttributeValue::Udata(addr) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `high_pc_to_u64`"),
    }
}
