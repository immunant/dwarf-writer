use gimli::write::{Address, AttributeValue, StringTable};

pub fn name_to_str<'a>(attr: &'a AttributeValue, strings: &'a StringTable) -> &'a [u8] {
    // TODO: This is missing some cases
    match attr {
        AttributeValue::String(s) => s,
        AttributeValue::StringRef(str_id) => strings.get(*str_id),
        _ => panic!("Unhandled `AttributeValue` variant in `name_to_str`"),
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
