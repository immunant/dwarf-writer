use crate::anvill;
use anyhow::Result;
use gimli::write::{Address, AttributeValue, StringTable};
use std::convert::TryInto;

pub fn name_to_anvill_ty(attr: &AttributeValue, strings: &StringTable) -> Option<anvill::Type> {
    let name: Result<anvill::Type> = name_to_bytes(attr, strings).try_into();
    name.ok()
}
pub fn name_to_bytes<'a>(attr: &'a AttributeValue, strings: &'a StringTable) -> &'a [u8] {
    // TODO: This is missing some cases
    match attr {
        AttributeValue::String(s) => s,
        AttributeValue::StringRef(str_id) => strings.get(*str_id),
        _ => panic!("Unhandled `AttributeValue` variant in `name_to_bytes`"),
    }
}

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
