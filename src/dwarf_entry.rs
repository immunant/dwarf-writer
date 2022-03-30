use crate::anvill::{AnvillFnMap, AnvillVarMap};
use crate::dwarf_attr::*;
use crate::elf::ELF;
use crate::str_bsi::StrFnMap;
use crate::types::{DwarfType, TypeMap};
use gimli::constants::*;
use gimli::write::{Address, AttributeValue, DebuggingInformationEntry, Unit, UnitEntryId, UnitId};
use log::trace;
use object::Object;
use std::ops::{Deref, DerefMut};

/// Reference to an entry in a `gimli::write::Unit`.
#[derive(Debug)]
pub struct EntryRef<'a> {
    elf: &'a mut ELF,
    // The entry's ID.
    id: UnitEntryId,
}

impl Deref for EntryRef<'_> {
    type Target = DebuggingInformationEntry;

    fn deref(&self) -> &Self::Target {
        let id = self.id;
        self.get_unit().get(id)
    }
}

impl DerefMut for EntryRef<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let id = self.id;
        self.get_mut_unit().get_mut(id)
    }
}

impl<'a> EntryRef<'a> {
    pub fn new(elf: &'a mut ELF, id: UnitEntryId) -> Self {
        EntryRef { elf, id }
    }

    fn unit_id(&self) -> UnitId {
        self.elf.dwarf.units.id(0)
    }

    fn get_unit(&self) -> &Unit {
        let root = self.unit_id();
        self.elf.dwarf.units.get(root)
    }

    fn get_mut_unit(&mut self) -> &mut Unit {
        let root = self.unit_id();
        self.elf.dwarf.units.get_mut(root)
    }

    fn new_sibling(&mut self, tag: DwTag) -> EntryRef {
        let parent = self
            .parent()
            .expect("`new_sibling` cannot be called on root entry");
        let sibling_id = self.get_mut_unit().add(parent, tag);
        EntryRef::new(self.elf, sibling_id)
    }

    fn new_child(&mut self, tag: DwTag) -> EntryRef {
        let id = self.id;
        let child_id = self.get_mut_unit().add(id, tag);
        EntryRef::new(self.elf, child_id)
    }

    /// Initializes a newly created subprogram entry with STR data.
    pub fn init_str_fn(&mut self, addr: u64, str_data: &mut StrFnMap, type_map: &TypeMap) {
        self.set(
            DW_AT_low_pc,
            AttributeValue::Address(Address::Constant(addr)),
        );
        self.update_str_fn(str_data, type_map)
    }

    /// Updates an existing function's subprogram entry with STR data.
    pub fn update_str_fn(&mut self, str_data: &mut StrFnMap, type_map: &TypeMap) {
        // Get function address to see if there's disassembly data for it
        let low_pc_attr = self
            .get(DW_AT_low_pc)
            .expect("No DW_AT_low_pc found in DW_TAG_subprogram entry");
        let start_address = low_pc_to_u64(low_pc_attr);

        let fn_data = str_data.remove(&start_address);
        if let Some(fn_data) = fn_data {
            // Update function name and source location
            if let Some(name) =
                self.update_name(fn_data.symbol_name.as_deref(), "FUN_", start_address)
            {
                self.set(DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
            }
            if let Some(file) = fn_data.file() {
                self.set(
                    DW_AT_decl_file,
                    AttributeValue::String(file.as_bytes().to_vec()),
                );
            }
            if let Some(line) = fn_data.line() {
                self.set(DW_AT_decl_line, AttributeValue::Data8(line));
            }

            // Update function parameters
            if let Some(new_params) = &fn_data.parameters() {
                // Delete all existing parameters
                let existing_params: Vec<_> = self
                    .children()
                    .filter_map(|&child_id| {
                        if self.get_unit().get(child_id).tag() == DW_TAG_formal_parameter {
                            Some(child_id)
                        } else {
                            None
                        }
                    })
                    .collect();
                for param in existing_params {
                    self.delete_child(param);
                }

                for param in new_params {
                    let mut param_entry = self.new_child(DW_TAG_formal_parameter);
                    if let Some(ref ty) = param.r#type {
                        let param_ty = DwarfType::from(ty);
                        let param_ty_id = type_map.get(&param_ty).unwrap_or_else(|| {
                            panic!("Parameter type {:?} not found in the type map", param_ty)
                        });
                        param_entry.set(DW_AT_type, AttributeValue::UnitRef(*param_ty_id));
                        param_entry.set(
                            DW_AT_name,
                            AttributeValue::String(param.name.as_bytes().to_vec()),
                        );
                    }
                }
            }

            // Update the function's local variables
            if let Some(local_vars) = &fn_data.local_vars() {
                for var in local_vars {
                    let mut var_entry = self.new_child(DW_TAG_variable);
                    if let Some(ref ty) = var.r#type {
                        let var_ty = DwarfType::from(ty);
                        let var_ty_id = type_map.get(&var_ty).unwrap_or_else(|| {
                            panic!("Variable type {:?} not found in the type map", var_ty)
                        });
                        var_entry.set(DW_AT_type, AttributeValue::UnitRef(*var_ty_id));
                        var_entry.set(
                            DW_AT_name,
                            AttributeValue::String(var.name.as_bytes().to_vec()),
                        );
                    }
                }
            }
        }
    }

    /// Initializes a newly created subprogram entry with Anvill data.
    pub fn init_anvill_fn(&mut self, addr: u64, anvill_data: &mut AnvillFnMap, type_map: &TypeMap) {
        self.set(
            DW_AT_low_pc,
            AttributeValue::Address(Address::Constant(addr)),
        );
        self.update_anvill_fn(anvill_data, type_map)
    }

    /// Updates an existing function's subprogram entry with Anvill data.
    pub fn update_anvill_fn(&mut self, anvill_data: &mut AnvillFnMap, type_map: &TypeMap) {
        // Get function address to see if there's disassembly data for it
        let low_pc_attr = self
            .get(DW_AT_low_pc)
            .expect("No DW_AT_low_pc found in DW_TAG_subprogram entry");
        let start_address = low_pc_to_u64(low_pc_attr);

        let fn_data = anvill_data.remove(&start_address);
        if let Some(fn_data) = fn_data {
            // Update function name
            if let Some(name) = self.update_name(fn_data.name, "FUN_", start_address) {
                self.set(DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
            }

            if let Some(ret_addr) = &fn_data.func.return_address {
                if let Some(loc) = &ret_addr.location {
                    self.set(DW_AT_return_addr, AttributeValue::from(loc));
                }
            }

            if let Some(no_ret) = fn_data.func.is_noreturn {
                self.set(DW_AT_noreturn, AttributeValue::Flag(no_ret));
            }

            self.set(DW_AT_prototyped, AttributeValue::Flag(true));

            if let Some(ret_vals) = &fn_data.func.return_values {
                if let Some(ret) = ret_vals.get(0) {
                    let ret_type = DwarfType::from(&ret.r#type);
                    let ret_type_entry_id = type_map.get(&ret_type).unwrap_or_else(|| {
                        panic!("Return type {:?} not found in the type map", ret_type)
                    });
                    self.set(DW_AT_type, AttributeValue::UnitRef(*ret_type_entry_id));
                }
            }

            if let Some(new_params) = &fn_data.func.parameters {
                // Delete all existing parameters
                let existing_params: Vec<_> = self
                    .children()
                    .filter_map(|&child_id| {
                        if self.get_unit().get(child_id).tag() == DW_TAG_formal_parameter {
                            Some(child_id)
                        } else {
                            None
                        }
                    })
                    .collect();
                for param in existing_params {
                    self.delete_child(param);
                }

                for param in new_params {
                    let mut param_entry = self.new_child(DW_TAG_formal_parameter);
                    if let Some(loc) = param.location() {
                        param_entry.set(DW_AT_location, AttributeValue::from(loc));
                    }
                    let param_ty = DwarfType::from(param.ty());
                    let param_ty_id = type_map.get(&param_ty).unwrap_or_else(|| {
                        panic!("Parameter type {:?} not found in the type map", param_ty)
                    });
                    param_entry.set(DW_AT_type, AttributeValue::UnitRef(*param_ty_id));
                    if let Some(param_name) = param.name() {
                        param_entry.set(
                            DW_AT_name,
                            AttributeValue::String(param_name.as_bytes().to_vec()),
                        );
                    };
                }
            }
        }
    }

    fn update_name(&mut self, new_name: Option<&str>, prefix: &str, addr: u64) -> Option<String> {
        let old_name = self.get(DW_AT_name);
        match (old_name, new_name) {
            (None, None) => Some(format!("{}{:08x}", prefix, addr)),
            (Some(_), None) => None,
            (_, Some(name)) => Some(name.to_string()),
        }
    }

    pub fn init_var(&mut self, addr: u64, anvill_data: &mut AnvillVarMap, type_map: &TypeMap) {
        self.set(DW_AT_location, addr_to_attr(addr));
        self.update_var(anvill_data, type_map);
    }

    /// Updates an existing variable's entry.
    pub fn update_var(&mut self, anvill_data: &mut AnvillVarMap, type_map: &TypeMap) {
        let location = self
            .get(DW_AT_location)
            .expect("No DW_AT_location found in DW_TAG_variable entry");
        // TODO: Ideally I'd get the address from `location` above then check if the
        // key's in the map, but I have to do it this way because the
        // `gimli::write::Operations` which make up an `Expression` are intentionally
        // kept private. There should be a way to tweak gimli to get the address of an
        // expression.
        let var_data = anvill_data
            .keys()
            .find(|&addr| addr_to_attr(*addr) == *location)
            .cloned()
            .map(|addr| anvill_data.remove(&addr))
            .flatten();
        if let Some(var_data) = var_data {
            // Update variable name
            if let Some(name) = self.update_name(var_data.name, "VAR_", var_data.var.address) {
                self.set(DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
            }

            // Update variale type
            let var_type = DwarfType::from(&var_data.var.r#type);
            let var_type_entry_id = type_map.get(&var_type).unwrap_or_else(|| {
                panic!("Variable type {:?} not found in the type map", var_type)
            });
            self.set(DW_AT_type, AttributeValue::UnitRef(*var_type_entry_id));
        }
    }

    pub fn init_type<'ty>(&mut self, ty: &'ty DwarfType, type_map: &mut TypeMap) {
        match ty {
            DwarfType::Primitive { name, size } => {
                assert_eq!(self.tag(), DW_TAG_base_type);
                self.set(DW_AT_name, AttributeValue::String(Vec::from(name.clone())));
                if let Some(size) = size {
                    self.set(DW_AT_byte_size, AttributeValue::Udata(*size));
                };
            },
            DwarfType::Pointer(pointee_type) => {
                assert_eq!(self.tag(), DW_TAG_pointer_type);
                let pointee = match type_map.get(pointee_type) {
                    // If the pointee type has already been seen
                    Some(pointee_ty_id) => *pointee_ty_id,
                    None => {
                        // If the pointee has not been seen, create its type and add it to the type
                        // map
                        let mut pointee_ty_entry = self.new_sibling(pointee_type.tag());
                        pointee_ty_entry.init_type(pointee_type, type_map);
                        trace!(
                            "Mapping type {:?} to entry {:?}",
                            *pointee_type.clone(),
                            pointee_ty_entry.id
                        );
                        type_map.insert(*pointee_type.clone(), pointee_ty_entry.id);

                        pointee_ty_entry.id
                    },
                };
                let ptr_size = if self.elf.object().is_64() { 8 } else { 4 };
                self.set(DW_AT_byte_size, AttributeValue::Udata(ptr_size));
                self.set(DW_AT_type, AttributeValue::UnitRef(pointee));
            },
            DwarfType::Typedef { .. } => {
                assert_eq!(self.tag(), DW_TAG_typedef);
            },
            DwarfType::Array { inner_type, len } => {
                assert_eq!(self.tag(), DW_TAG_array_type);
                let inner = match type_map.get(inner_type) {
                    Some(id) => *id,
                    None => {
                        let mut inner_ty_entry = self.new_sibling(inner_type.tag());
                        inner_ty_entry.init_type(inner_type, type_map);
                        type_map.insert(*inner_type.clone(), inner_ty_entry.id);
                        inner_ty_entry.id
                    },
                };
                self.set(DW_AT_type, AttributeValue::UnitRef(inner));
                let mut array_size = self.new_child(DW_TAG_subrange_type);
                if let Some(len) = len {
                    // TODO: Try encoding the size with less space
                    array_size.set(DW_AT_upper_bound, AttributeValue::Data8(*len));
                };
            },
            DwarfType::Struct(_) => {
                assert_eq!(self.tag(), DW_TAG_structure_type);
            },
            DwarfType::Function {
                return_type,
                args: _,
            } => {
                assert_eq!(self.tag(), DW_TAG_subroutine_type);
                let ret = match type_map.get(return_type) {
                    Some(ret_ty_id) => *ret_ty_id,
                    None => {
                        let mut ret_ty_entry = self.new_sibling(return_type.tag());
                        ret_ty_entry.init_type(return_type, type_map);
                        type_map.insert(*return_type.clone(), ret_ty_entry.id);
                        ret_ty_entry.id
                    },
                };
                self.set(DW_AT_type, AttributeValue::UnitRef(ret));
            },
        }
    }
}
