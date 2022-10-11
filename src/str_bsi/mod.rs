use crate::types::{CanonicalTypeName, DwarfType};
use crate::InputFile;
use crate::Opt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

impl InputFile for StrBsiInput {}

impl StrBsiInput {
    pub fn data(&self, cfg: &Opt) -> StrBsiData {
        let fn_map = if cfg.omit_functions {
            HashMap::new()
        } else {
            self.functions.iter().map(|f| (f.address, f)).collect()
        };
        let header_bytes = base64::decode(&self.header_file_b64).unwrap();
        let header = String::from_utf8(header_bytes).unwrap();
        StrBsiData { fn_map, header }
    }
}

pub type StrFnMap<'a> = HashMap<u64, &'a Function>;

pub struct StrBsiData<'a> {
    pub fn_map: StrFnMap<'a>,
    pub header: String,
}

impl Function {
    pub fn parameters(&self, header: &str) -> Option<Vec<NamedVariable>> {
        let name = self.symbol_name.as_ref()?;
        let start = header.find(&(name.to_owned() + "("))?;
        let fn_name = name.len() + 1;
        let end = header[start..].find(')')?;
        let fn_decl = &header[start + fn_name..start + end];
        let args = fn_decl.split(',');
        let mut params = Vec::new();
        for arg in args {
            if !arg.ends_with("...") && !arg.is_empty() {
                let name = arg.split(' ').last().map(|s| s.to_owned());
                if name.is_none() {
                    continue
                }
                let name = name.unwrap();
                let param = NamedVariable { name, r#type: None };
                params.push(param);
            }
        }
        Some(params)
    }
}

pub type Address = u64;
pub type Register = String;
pub type Type = String;
pub type VarId = String;

/// Represents a single STR BSI input file.
#[derive(Serialize, Deserialize, Debug)]
pub struct StrBsiInput {
    functions: Vec<Function>,
    header_file_b64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Function {
    #[serde(rename = "source_match")]
    pub symbol_name: Option<String>,
    calling_convention: Option<String>,
    return_registers: Vec<Register>,
    clobbered_registers: Vec<Register>,
    address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceMatch {
    confidence: u32,
    file: Option<String>,
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
    pub name: String,
    pub r#type: Option<Type>,
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
