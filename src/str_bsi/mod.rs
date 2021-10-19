use crate::types::{CanonicalTypeName, DwarfType};
use crate::InputFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

impl InputFile for StrBsiInput {}

impl StrBsiInput {
    pub fn data(&self) -> StrBsiData {
        let fn_map = self
            .functions
            .iter()
            .filter_map(|(addr, f)| {
                let confidence = f.source_match.as_ref().map(|sm| sm.confidence).unwrap_or(0);
                if confidence == 0 {
                    None
                } else {
                    let addr = match addr.strip_prefix("0x") {
                        Some(hex_addr) => u64::from_str_radix(hex_addr, 16),
                        None => u64::from_str(addr),
                    }
                    .unwrap_or_else(|_| panic!("Unable to parse {} into a u64", addr));
                    Some((addr, f))
                }
            })
            .collect();
        let dwarf_types = self.types().iter().map(|&t| t.into()).collect();
        StrBsiData {
            fn_map,
            types: dwarf_types,
        }
    }

    pub fn types(&self) -> Vec<&Type> {
        let mut types = Vec::new();
        for (_, func) in &self.functions {
            if let Some(sm) = &func.source_match {
                if sm.confidence == 1 {
                    types.append(&mut sm.types());
                }
            }
        }
        types.sort();
        types.dedup();
        types
    }
}

pub struct StrBsiData<'a> {
    pub fn_map: HashMap<u64, &'a Function>,
    pub types: Vec<DwarfType>,
}

pub type Address = String;
pub type Register = String;
pub type Type = String;
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
    return_value: UnnamedVariable,
    parameters: Option<HashMap<VarId, NamedVariable>>,
    local_variables: Option<HashMap<VarId, NamedVariable>>,
}

impl SourceMatch {
    pub fn types(&self) -> Vec<&Type> {
        let mut types = Vec::new();
        if let Some(parameters) = &self.parameters {
            for (_, var) in parameters {
                if let Some(ty) = &var.r#type {
                    types.push(ty);
                }
            }
        };
        if let Some(local_variables) = &self.local_variables {
            for (_, var) in local_variables {
                if let Some(ty) = &var.r#type {
                    types.push(ty);
                }
            }
        };
        if let Some(ty) = &self.return_value.r#type {
            types.push(ty);
        };
        types
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnnamedVariable {
    r#type: Option<Type>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NamedVariable {
    name: String,
    r#type: Option<Type>,
}

impl From<&Type> for DwarfType {
    fn from(str_ty: &Type) -> DwarfType {
        if let Some(referent_ty) = str_ty.strip_suffix("*") {
            DwarfType::new_pointer(DwarfType::from(&String::from(referent_ty)))
        } else if let Some(inner_ty) = str_ty.strip_suffix("[]") {
            DwarfType::new_array(DwarfType::from(&String::from(inner_ty)), None)
        } else if let Some(inner_ty) = str_ty.strip_suffix("]") {
            let mut inner_ty = inner_ty.split('[').collect::<Vec<_>>();
            let array_len = inner_ty
                .pop()
                .map(|ty| u64::from_str(ty).ok())
                .flatten()
                .unwrap_or_else(|| panic!("Unable to parse type {:?}", inner_ty));
            let array_ty = inner_ty.join("");
            DwarfType::new_array(DwarfType::from(&array_ty), Some(array_len))
        } else {
            DwarfType::new_primitive(CanonicalTypeName::from(str_ty.as_bytes().to_vec()), None)
        }
    }
}
