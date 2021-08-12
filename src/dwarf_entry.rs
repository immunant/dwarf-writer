use crate::anvill::{AnvillFnMap, AnvillVarMap};
use crate::dwarf_attr::*;
use crate::elf::ELF;
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

    /// Initializes a newly created subprogram entry.
    pub fn init_fn(&mut self, addr: u64, anvill_data: &mut AnvillFnMap, type_map: &TypeMap) {
        self.set(
            DW_AT_low_pc,
            AttributeValue::Address(Address::Constant(addr)),
        );
        self.update_fn(anvill_data, type_map)
    }

    /// Updates an existing function's subprogram entry.
    pub fn update_fn(&mut self, anvill_data: &mut AnvillFnMap, type_map: &TypeMap) {
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

            self.set(
                DW_AT_return_addr,
                AttributeValue::from(&fn_data.func.return_address.location),
            );

            if let Some(no_ret) = fn_data.func.is_noreturn {
                self.set(DW_AT_noreturn, AttributeValue::Flag(no_ret));
            }

            self.set(DW_AT_prototyped, AttributeValue::Flag(true));

            if let Some(ret_vals) = &fn_data.func.return_values {
                let ret_type = DwarfType::from(&ret_vals[0].r#type);
                let ret_type_entry_id = type_map
                    .get(&ret_type)
                    .expect("All types should be in the type map");
                self.set(DW_AT_type, AttributeValue::UnitRef(*ret_type_entry_id));
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
                    param_entry.set(DW_AT_location, AttributeValue::from(param.location()));
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
            let var_type_entry_id = type_map
                .get(&var_type)
                .expect("All types should be in the type map");
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
                        pointee_ty_entry.init_type(&pointee_type, type_map);
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
            DwarfType::Typedef(..) => (),
            DwarfType::Array { .. } => (),
            DwarfType::Struct => (),
            DwarfType::Function => (),
        }
    }
}
