use crate::anvill;
use anyhow::Result;
use gimli::write::{Address, AttributeValue, Expression, StringTable};
use std::convert::TryInto;

pub fn dwarf_location(location: &anvill::Tagged) -> AttributeValue {
    use anvill::{Tagged, X86Register};

    fn reg_num(r: &anvill::Register) -> u16 {
        use anvill::Register;
        match r {
            Register::X86(r) => *r as u16,
            Register::ARM(r) => *r as u16,
            Register::SPARC(r) => *r as u16,
        }
    }

    let mut expr = Expression::new();
    match location {
        Tagged::register(reg) => expr.op_reg(gimli::Register(reg_num(reg))),
        Tagged::memory { register, offset } => {
            expr.op_breg(gimli::Register(reg_num(register)), *offset)
        },
    }
    AttributeValue::Exprloc(expr)
}

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
