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

        let reg_string = serde_json::to_string(self)
            .expect("Couldn't serialize `anvill::Register` to `String`")
            .trim_matches('"')
            .to_ascii_lowercase();
        let name_to_register = match self {
            Register::X86(_) => gimli::X86_64::name_to_register,
            Register::ARM(_) => gimli::Arm::name_to_register,
            Register::SPARC(r) => {
                return gimli::Register(*r as u16)
            },
        };
        let reg = name_to_register(&reg_string)
            .unwrap_or_else(|| panic!("Couldn't map {:?} to `gimli::Register`", reg_string));
        reg
    }
}
