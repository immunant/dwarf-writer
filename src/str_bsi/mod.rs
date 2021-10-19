use crate::InputFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

impl InputFile for StrBsiInput {}

impl StrBsiInput {
    pub fn data(&self) -> StrBsiData {
        self.functions
            .iter()
            .map(|(addr, f)| {
                let addr = match addr.strip_prefix("0x") {
                    Some(hex_addr) => u64::from_str_radix(hex_addr, 16),
                    None => u64::from_str(addr),
                }
                .unwrap_or_else(|_| panic!("Unable to parse {} into a u64", addr));
                (addr, f)
            })
            .collect()
    }
}

pub type StrBsiData<'a> = HashMap<u64, &'a Function>;
pub type Register = String;
pub type Address = String;
pub type VarId = String;

/// Represents a single STR BSI input file.
#[derive(Serialize, Deserialize, Debug)]
pub struct StrBsiInput {
    functions: HashMap<Address, Function>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Function {
    symbol_name: String,
    calling_convention: Option<String>,
    return_registers: Vec<Register>,
    clobbered_registers: Vec<Register>,
    source_match: Option<SourceMatch>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceMatch {
    confidence: u32,
    file: String,
    line: Option<u64>,
    function: String,
    return_value: UnnamedType,
    parameters: Option<HashMap<VarId, NamedType>>,
    local_variables: Option<HashMap<VarId, NamedType>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnnamedType {
    r#type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NamedType {
    name: String,
    r#type: Option<String>,
}
