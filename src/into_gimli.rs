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

        let name_to_register = match self {
            Register::X86(_) => gimli::X86_64::name_to_register,
            Register::ARM(_) => gimli::Arm::name_to_register,
            Register::SPARC(r) => return gimli::Register(*r as u16),
        };
        let lower_case = match self {
            Register::X86(_) => true,
            Register::ARM(_) => false,
            _ => unreachable!("SPARC currently doesn't use `name_to_register`"),
        };
        let reg_string =
            serde_json::to_string(self).expect("Couldn't serialize `anvill::Register` to `String`");
        let reg_string = if lower_case {
            reg_string.trim_matches('"').to_ascii_lowercase()
        } else {
            reg_string.trim_matches('"').to_ascii_uppercase()
        };
        name_to_register(&reg_string)
            .unwrap_or_else(|| panic!("Couldn't map {:?} to `gimli::Register`", reg_string))
    }
}
