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
        match self {
            Register::X86(r) => {
                let reg_string = serde_json::to_string(self)
                    .unwrap_or_else(|_| panic!("Couldn't serialize X86 register {:?}", r));
                println!("{:?}", reg_string);
                let gimli_reg_name = match reg_string.trim_matches('"') {
                    "RSP" => "RA".to_string(),
                    s => s.to_ascii_lowercase(),
                };
                let reg = gimli::X86_64::name_to_register(&gimli_reg_name).unwrap_or_else(|| {
                    panic!(
                        "Couldn't map X86 register name {:?} to gimli::Register",
                        gimli_reg_name
                    )
                });
                reg
            },
            Register::ARM(r) => gimli::Register(*r as u16),
            Register::SPARC(r) => gimli::Register(*r as u16),
        }
    }
}
