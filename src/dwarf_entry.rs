use crate::anvill;
use crate::anvill::AnvillFnMap;
use crate::dwarf_attr::*;
use gimli::constants;
use gimli::constants::*;
use gimli::write::{Address, AttributeValue, StringTable, Unit, UnitEntryId};

/// Reference to an entry in a `gimli::write::Unit`.
#[derive(Debug)]
pub struct EntryRef<'a> {
    // The unit containing the entry.
    unit: &'a mut Unit,
    // The entry's ID.
    self_id: UnitEntryId,

    // Miscellaneous DWARF info
    strings: &'a StringTable,
}

impl<'a> EntryRef<'a> {
    pub fn new(unit: &'a mut Unit, self_id: UnitEntryId, strings: &'a StringTable) -> Self {
        EntryRef {
            unit,
            self_id,
            strings,
        }
    }
    /// Initializes a newly created subprogram entry.
    pub fn create_fn(&mut self, addr: u64, anvill_data: &mut AnvillFnMap) {
        let entry = self.unit.get_mut(self.self_id);
        entry.set(
            DW_AT_low_pc,
            AttributeValue::Address(Address::Constant(addr)),
        );
        self.update_fn(anvill_data)
    }

    /// Updates an existing function's subprogram entry.
    pub fn update_fn(&mut self, anvill_data: &mut AnvillFnMap) {
        let EntryRef {
            unit,
            self_id,
            strings: _,
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

            // Update function parameters
            if let Some(new_params) = fn_data.func.parameters() {
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
                    let param_entry = unit.get_mut(param_id);
                    if let Some(param_name) = param.name() {
                        param_entry.set(
                            DW_AT_name,
                            AttributeValue::String(param_name.as_bytes().to_vec()),
                        );
                    };
                }
                // Mark the subprogram entry as prototyped
                let entry = unit.get_mut(self_id);
                entry.set(DW_AT_prototyped, AttributeValue::Flag(true));
            }
        }
    }

    /*
    /// Checks if the given anvill type matches the type entry.
    ///
    /// A type may have various string representations (e.g. `bool` from
    /// `stdbool.h` expands to `_Bool` while C++/rust's boolean is `bool`).
    /// meaning this method may produce false negatives. In the case of a
    /// false negative, a new type entry will be created for the incorrectly
    /// identified type, but subsequent comparisons between the new entry and
    /// the type will always succeed.
    pub fn type_matches(&self, ty: &anvill::Type) -> bool {
        let entry = self.unit.get(self.self_id);
        match entry.tag() {
            constants::DW_TAG_base_type => match entry.get(DW_AT_name) {
                Some(name_attr) => {
                    if let Some(name) = name_to_anvill_ty(name_attr, self.strings) {
                        name == *ty
                    } else {
                        false
                    }
                },
                None => false,
            },
            constants::DW_TAG_pointer_type => false,
            _ => false,
        }
    }

    pub fn create_type(&mut self, ty: &anvill::Type) {
        //let entry = self.unit.get_mut(self.self_id);
        //let ty_name: &[u8] = ty.into();
        //entry.set(DW_AT_name, AttributeValue::String(ty_name.to_vec()));
        //// TODO: DW_AT_encoding
        //entry.set(DW_AT_byte_size, AttributeValue::Data1(ty.size()));
    }
    */
}
