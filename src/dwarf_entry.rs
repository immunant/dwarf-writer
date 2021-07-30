use crate::anvill;
use crate::anvill::AnvillFnMap;
use crate::dwarf_attr::*;
use crate::types::TypeMap;
use gimli::constants::*;
use gimli::write::{Address, AttributeValue, Reference, Unit, UnitEntryId, UnitId};

/// Reference to an entry in a `gimli::write::Unit`.
#[derive(Debug)]
pub struct EntryRef<'a> {
    // The unit containing the entry.
    unit: &'a mut Unit,
    // The unit's ID.
    unit_id: UnitId,
    // The entry's ID.
    self_id: UnitEntryId,
}

impl<'a> EntryRef<'a> {
    pub fn new(unit: &'a mut Unit, unit_id: UnitId, self_id: UnitEntryId) -> Self {
        EntryRef {
            unit,
            unit_id,
            self_id,
        }
    }

    /// Initializes a newly created subprogram entry.
    pub fn create_fn(&mut self, addr: u64, anvill_data: &mut AnvillFnMap, type_map: &TypeMap) {
        let entry = self.unit.get_mut(self.self_id);
        entry.set(
            DW_AT_low_pc,
            AttributeValue::Address(Address::Constant(addr)),
        );
        self.update_fn(anvill_data, type_map)
    }

    /// Updates an existing function's subprogram entry.
    pub fn update_fn(&mut self, anvill_data: &mut AnvillFnMap, type_map: &TypeMap) {
        let EntryRef {
            unit,
            self_id,
            unit_id,
        } = self;
        let self_id = *self_id;
        let entry = unit.get(self_id);

        // Get this function's address from the existing DWARF data
        let low_pc_attr = entry
            .get(DW_AT_low_pc)
            .expect("No DW_AT_low_pc found in DW_TAG_subprogram entry");
        let start_address = low_pc_to_u64(low_pc_attr);

        // Search for this function's anvill data by start address
        let fn_data = anvill_data.remove(&start_address);
        if let Some(fn_data) = fn_data {
            // Update function name overwriting any existing DW_AT_name
            let existing_name = entry.get(DW_AT_name);
            let new_name = match (existing_name, fn_data.name) {
                (None, None) => Some(format!("__unnamed_fn_{}", start_address)),
                (Some(_), None) => None,
                (_, Some(name)) => Some(name.to_string()),
            };
            if let Some(name) = new_name {
                let entry = unit.get_mut(self_id);
                entry.set(DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
            }

            let entry = unit.get_mut(self_id);
            entry.set(
                DW_AT_return_addr,
                dwarf_location(&fn_data.func.return_address.location),
            );

            // TODO: This is only supported for DWARF 5, but ghidra doesn't complain when
            // it's used with DWARF 4. I should double check with other tools.
            if let Some(no_ret) = fn_data.func.is_noreturn {
                entry.set(DW_AT_noreturn, AttributeValue::Flag(no_ret));
            }

            // Mark the subprogram entry as prototyped
            let entry = unit.get_mut(self_id);
            entry.set(DW_AT_prototyped, AttributeValue::Flag(true));

            if let Some(ret_vals) = &fn_data.func.return_values {
                // TODO: Handle multiple ret values
                entry.set(DW_AT_type, AttributeValue::Data1(ret_vals[0].r#type.size()));
                let type_name = ret_vals[0].r#type.name();
                let type_id = type_map.get(&type_name).unwrap_or_else(|| {
                    panic!("Type {:?} was not found in the type map", type_name)
                });
                // TODO: Make a sensible way to get the compilation unit ID
                let type_ref = Reference::Entry(*unit_id, *type_id);
                entry.set(DW_AT_type, AttributeValue::DebugInfoRef(type_ref));
            }

            // Update function parameters
            if let Some(new_params) = &fn_data.func.parameters {
                // Clear all existing parameters to avoid duplicates
                let entry = unit.get(self_id);
                let existing_params: Vec<_> = entry
                    .children()
                    .filter_map(|&child_id| {
                        let child_entry = unit.get(child_id);
                        let child_tag = child_entry.tag();
                        if child_tag == DW_TAG_formal_parameter {
                            Some(child_id)
                        } else {
                            None
                        }
                    })
                    .collect();
                let entry = unit.get_mut(self_id);
                for param in existing_params {
                    entry.delete_child(param);
                }

                for param in new_params {
                    let param_id = unit.add(self_id, DW_TAG_formal_parameter);
                    let entry = unit.get_mut(param_id);
                    entry.set(DW_AT_location, dwarf_location(&param.location()));
                    if let Some(param_name) = param.name() {
                        entry.set(
                            DW_AT_name,
                            AttributeValue::String(param_name.as_bytes().to_vec()),
                        );
                    };
                }
            }
        }
    }

    pub fn create_type(&mut self, ty: &anvill::Type) {
        let entry = self.unit.get_mut(self.self_id);
        entry.set(DW_AT_name, AttributeValue::String(Vec::from(ty.name())));
        entry.set(DW_AT_byte_size, AttributeValue::Data1(ty.size()));
        // TODO: Set DW_AT_encoding
    }
}
