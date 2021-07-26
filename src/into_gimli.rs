use crate::anvill;

/// Generic trait for converting to gimli-specific types.
pub trait IntoGimli<T> {
    fn into_gimli(self) -> T;
}

impl IntoGimli<gimli::RunTimeEndian> for object::endian::Endianness {
    fn into_gimli(self) -> gimli::RunTimeEndian {
        use gimli::RunTimeEndian as gimli;
        use object::endian::Endianness as obj;
        match self {
            obj::Little => gimli::Little,
            obj::Big => gimli::Big,
        }
    }
}

impl IntoGimli<gimli::Register> for &anvill::Register {
    fn into_gimli(self) -> gimli::Register {
        use anvill::Register;
        let reg_num = match self {
            Register::X86(r) => *r as u16,
            Register::ARM(r) => *r as u16,
            Register::SPARC(r) => *r as u16,
        };
        gimli::Register(reg_num)
    }
}
