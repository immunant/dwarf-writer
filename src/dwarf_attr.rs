use crate::anvill;
use crate::dwarf_entry::EntryRef;
use crate::into_gimli::IntoGimli;
use gimli::write::{Address, AttributeValue, Expression, StringTable, UnitEntryId};

impl From<&anvill::TaggedLocation> for AttributeValue {
    fn from(location: &anvill::TaggedLocation) -> AttributeValue {
        use anvill::TaggedLocation;

        let mut expr = Expression::new();
        match location {
            TaggedLocation::register(reg) => expr.op_reg(reg.into_gimli()),
            TaggedLocation::memory { register, offset } => {
                expr.op_breg(register.into_gimli(), *offset)
            },
        }
        AttributeValue::Exprloc(expr)
    }
}

impl<'a> From<&EntryRef<'a>> for AttributeValue {
    fn from(entry_ref: &EntryRef) -> AttributeValue {
        AttributeValue::UnitRef(entry_ref.id())
    }
}

pub fn addr_to_attr(addr: u64) -> AttributeValue {
    let mut expr = Expression::new();
    expr.op_addr(Address::Constant(addr));
    AttributeValue::Exprloc(expr)
}

pub fn name_as_bytes<'a>(attr: &'a AttributeValue, strings: &'a StringTable) -> &'a [u8] {
    // TODO: This is missing some cases
    match attr {
        AttributeValue::String(s) => s,
        AttributeValue::StringRef(str_id) => strings.get(*str_id),
        _ => panic!("Unhandled `AttributeValue` variant in `name_as_bytes`"),
    }
}

pub fn low_pc_to_u64(attr: &AttributeValue) -> u64 {
    // TODO: Handle Address::Symbol
    match attr {
        AttributeValue::Address(Address::Constant(addr)) => *addr,
        AttributeValue::Udata(addr) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `low_pc_to_u64`"),
    }
}

#[allow(dead_code)]
pub fn attr_to_u8(attr: &AttributeValue) -> u8 {
    match attr {
        AttributeValue::Data1(b) => *b,
        _ => panic!(
            "Unhandled `AttributeValue` variant {:?} in `attr_to_u8`",
            attr
        ),
    }
}

pub fn attr_to_u64(attr: &AttributeValue) -> u64 {
    match attr {
        AttributeValue::Data8(b) => *b,
        AttributeValue::Udata(b) => *b,
        _ => panic!(
            "Unhandled `AttributeValue` variant {:?} in `attr_to_u64`",
            attr
        ),
    }
}

pub fn attr_to_entry_id(attr: &AttributeValue) -> UnitEntryId {
    match attr {
        AttributeValue::UnitRef(r) => *r,
        _ => panic!(""),
    }
}
