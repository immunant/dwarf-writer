use crate::anvill::AnvillFnMap;
use crate::dwarf_attr::*;
use crate::types::{DwarfType, TypeMap};
use gimli::constants::*;
use gimli::write::{Address, AttributeValue, DebuggingInformationEntry, Dwarf, Reference, Unit,
                   UnitEntryId, UnitId};
use log::debug;
use std::ops::{Deref, DerefMut};

/// Reference to an entry in a `gimli::write::Unit`.
#[derive(Debug)]
pub struct EntryRef<'a> {
    dwarf: &'a mut Dwarf,
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

impl<'a> From<&EntryRef<'a>> for AttributeValue {
    fn from(entry_ref: &EntryRef) -> AttributeValue {
        AttributeValue::DebugInfoRef(Reference::Entry(entry_ref.unit_id(), entry_ref.id))
    }
}

impl<'a> EntryRef<'a> {
    pub fn new(dwarf: &'a mut Dwarf, id: UnitEntryId) -> Self {
        EntryRef { dwarf, id }
    }

    fn unit_id(&self) -> UnitId {
        self.dwarf.units.id(0)
    }

    fn get_unit(&self) -> &Unit {
        let root = self.unit_id();
        self.dwarf.units.get(root)
    }

    fn get_mut_unit(&mut self) -> &mut Unit {
        let root = self.unit_id();
        self.dwarf.units.get_mut(root)
    }

    fn new_sibling(&mut self, tag: DwTag) -> EntryRef {
        let parent = self
            .parent()
            .expect("`new_sibling` cannot be called on root entry");
        let sibling_id = self.get_mut_unit().add(parent, tag);
        EntryRef::new(self.dwarf, sibling_id)
    }

    fn new_child(&mut self, tag: DwTag) -> EntryRef {
        let id = self.id;
        let child_id = self.get_mut_unit().add(id, tag);
        EntryRef::new(self.dwarf, child_id)
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
            let old_name = self.get(DW_AT_name);
            let new_name = match (old_name, fn_data.name) {
                (None, None) => Some(format!("FUN_{:08x}", start_address)),
                (Some(_), None) => None,
                (_, Some(name)) => Some(name.to_string()),
            };
            if let Some(name) = new_name {
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
                let ret_type_entry = Reference::Entry(self.unit_id(), *ret_type_entry_id);
                self.set(DW_AT_type, AttributeValue::DebugInfoRef(ret_type_entry));
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

    pub fn init_type<'ty>(&mut self, ty: &'ty DwarfType, type_map: &mut TypeMap) {
        match ty {
            DwarfType::Primitive { name, size } => {
                assert_eq!(self.tag(), DW_TAG_base_type);
                self.set(DW_AT_name, AttributeValue::String(Vec::from(name.clone())));
                if let Some(size) = size {
                    self.set(DW_AT_byte_size, AttributeValue::Data1(*size));
                };
            },
            DwarfType::Pointer(pointee_type) => {
                assert_eq!(self.tag(), DW_TAG_pointer_type);
                match type_map.get(pointee_type) {
                    // If the pointee type has already been seen
                    Some(pointee_ty_id) => {
                        // TODO: Handle setting pointer size. May need to reference ELF in EntryRef
                        //self.set(DW_AT_byte_size, AttributeValue::Data1(8));
                        let pointee_ty_entry = Reference::Entry(self.unit_id(), *pointee_ty_id);
                        self.set(DW_AT_type, AttributeValue::DebugInfoRef(pointee_ty_entry));
                    },
                    None => {
                        // If the pointee type has not been seen, create the type and add it to the
                        // type map
                        let mut pointee_ty_entry = self.new_sibling(pointee_type.tag());
                        pointee_ty_entry.init_type(&pointee_type, type_map);
                        let pointee_ty_attr_value = AttributeValue::from(&pointee_ty_entry);
                        pointee_ty_entry.set(DW_AT_type, pointee_ty_attr_value);
                    },
                }
            },
            DwarfType::Array { .. } => (),
            DwarfType::Struct => (),
            DwarfType::Function => (),
            _ => (),
        }
    }
}
