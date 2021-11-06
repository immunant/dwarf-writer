use crate::anvill::AnvillData;
use crate::dwarf_attr::{attr_to_entry_id, attr_to_u64, name_as_bytes};
use crate::dwarf_entry::EntryRef;
use crate::elf::ELF;
use crate::functions::FnMap;
use crate::str_bsi::StrBsiData;
use crate::types::{CanonicalTypeName, DwarfType, TypeMap};
use gimli::constants;
use gimli::constants::*;
use gimli::write::{DebuggingInformationEntry, LineProgram, StringTable, Unit, UnitEntryId, UnitId};
use gimli::{Encoding, Format};
use log::trace;
use object::Object;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

pub struct DwarfUnitRef<'a> {
    elf: &'a mut ELF,
    // The unit's ID.
    id: UnitId,
}

impl Deref for DwarfUnitRef<'_> {
    type Target = Unit;

    fn deref(&self) -> &Self::Target {
        let id = self.id;
        self.elf.dwarf.units.get(id)
    }
}

impl DerefMut for DwarfUnitRef<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let id = self.id;
        self.elf.dwarf.units.get_mut(id)
    }
}

impl<'a> DwarfUnitRef<'a> {
    /// Creates a DWARF unit if none exists in the `ELF`.
    pub fn new(elf: &'a mut ELF) -> Self {
        let num_units = elf.dwarf.units.count();
        if num_units == 0 {
            let is_64_bit = elf.object().is_64();
            let format = if is_64_bit {
                Format::Dwarf64
            } else {
                Format::Dwarf32
            };
            let encoding = Encoding {
                address_size: format.word_size(),
                format,
                version: 4,
            };
            let line_program = LineProgram::none();
            let unit = Unit::new(encoding, line_program);
            elf.dwarf.units.add(unit);
        }
        let id = elf.dwarf.units.id(0);
        DwarfUnitRef { elf, id }
    }

    fn new_entry(&mut self, parent: UnitEntryId, tag: DwTag) -> EntryRef {
        let id = self.add(parent, tag);
        self.entry_ref(id)
    }

    fn entry_ref(&mut self, id: UnitEntryId) -> EntryRef {
        EntryRef::new(self.elf, id)
    }

    fn strings(&self) -> &StringTable {
        &self.elf.dwarf.strings
    }

    /// Creates a type map from existing DWARF debug info. Returns an empty map
    /// if no debug info exists.
    pub fn create_type_map(&self) -> TypeMap {
        /// Searches the type map for the pointee of a type entry referencing
        /// another type. Returns `None` if the pointee has not been inserted
        /// into the type map.
        fn get_type_pointee(
            entry: &DebuggingInformationEntry, type_map: &mut TypeMap,
        ) -> Option<DwarfType> {
            if let Some(pointee_type) = entry.get(DW_AT_type) {
                let pointee_id = attr_to_entry_id(pointee_type);
                let pointee =
                    type_map
                        .iter()
                        .find_map(|(k, &v)| if v == pointee_id { Some(k) } else { None });
                match pointee {
                    Some(pointee) => {
                        trace!("Found pointee type {:?} in type map", pointee_type);
                        Some(pointee.clone())
                    },
                    None => {
                        trace!("Did not find pointee in type map");
                        None
                    },
                }
            } else {
                None
            }
        }

        trace!("Creating a type map");
        let mut type_map = HashMap::new();
        let root = self.root();

        let mut children: Vec<_> = self.get(root).children().cloned().collect();
        while !children.is_empty() {
            let current_iter: Vec<_> = children.drain(..).collect();
            for child in current_iter {
                let entry = self.get(child);

                match entry.tag() {
                    constants::DW_TAG_base_type => {
                        trace!("Found a base_type entry");
                        if let Some(name) = entry.get(DW_AT_name) {
                            let name = CanonicalTypeName::from(
                                name_as_bytes(name, self.strings()).to_vec(),
                            );
                            let size = entry.get(DW_AT_byte_size).map(|s| attr_to_u64(s));

                            trace!(
                                "Inserting base type named {:?} of size {:?} into type map",
                                name,
                                size
                            );
                            type_map.insert(DwarfType::new_primitive(name, size), child);
                        };
                    },
                    constants::DW_TAG_pointer_type => {
                        trace!("Found a pointer type entry");
                        match get_type_pointee(entry, &mut type_map) {
                            Some(pointee) => {
                                type_map.insert(DwarfType::new_pointer(pointee), child);
                            },
                            None => children.push(child),
                        };
                    },
                    constants::DW_TAG_typedef => {
                        trace!("Found a typedef entry");
                        let name = entry
                            .get(DW_AT_name)
                            .expect("Typedef entry should have a name");
                        match get_type_pointee(entry, &mut type_map) {
                            Some(ref_type) => {
                                type_map.insert(
                                    DwarfType::new_typedef(
                                        name_as_bytes(name, self.strings()).to_vec().into(),
                                        ref_type,
                                    ),
                                    child,
                                );
                            },
                            None => children.push(child),
                        }
                    },
                    constants::DW_TAG_array_type => {
                        trace!("Found an array type entry");
                        let len = entry.children().find_map(|&id| {
                            let child = self.get(id);
                            if child.tag() == DW_TAG_subrange_type {
                                child.get(DW_AT_upper_bound).map(attr_to_u64)
                            } else {
                                None
                            }
                        });
                        match get_type_pointee(entry, &mut type_map) {
                            Some(pointee) => {
                                type_map.insert(DwarfType::new_array(pointee, len), child);
                            },
                            None => children.push(child),
                        }
                    },
                    constants::DW_TAG_structure_type => {},
                    constants::DW_TAG_subroutine_type => {
                        trace!("Found a subroutine type entry");
                        match get_type_pointee(entry, &mut type_map) {
                            Some(pointee) => {
                                type_map
                                    .insert(DwarfType::new_function(pointee, Vec::new()), child);
                            },
                            None => children.push(child),
                        }
                    },
                    _ => (),
                }
            }
        }

        trace!("Created a type map from {} existing types", type_map.len());
        type_map
    }

    fn update_types(&mut self, types: Vec<DwarfType>, type_map: &mut TypeMap) {
        trace!("Processing anvill types");
        for ty in types {
            if !type_map.contains_key(&ty) {
                let mut ty_entry = self.new_entry(self.root(), ty.tag());
                ty_entry.init_type(&ty, type_map);

                // Update the type map with the new type
                trace!("Mapping type {:?} to entry {:?}", ty, ty_entry.id());
                type_map.insert(ty, ty_entry.id());
            }
        }
    }

    fn for_each_entry<F: FnMut(&mut Self, &UnitEntryId)>(&mut self, mut f: F) {
        let root = self.root();
        let mut children: Vec<_> = self.get(root).children().cloned().collect();

        while !children.is_empty() {
            let current_generation: Vec<_> = children.drain(..).collect();
            for entry_id in current_generation {
                let entry = self.get(entry_id);

                let mut next_generation = entry.children().cloned().collect();
                children.append(&mut next_generation);

                f(self, &entry_id);
            }
        }
    }

    pub fn process(&mut self, mut fn_map: FnMap, type_map: &mut TypeMap) {
        let mut types: Vec<_> = fn_map.iter().map(|(_, f)| f.types()).flatten().collect();
        types.sort();
        types.dedup();
        self.update_types(types, type_map);

        self.for_each_entry(|dwarf, &entry_id| {
            let entry = dwarf.get(entry_id);
            if entry.tag() == constants::DW_TAG_subprogram {
                let mut fn_entry = dwarf.entry_ref(entry_id);
                fn_entry.update_fn(&mut fn_map, type_map);
            }
        });

        let root = self.root();
        let remaining_fn_addrs: Vec<_> = fn_map.keys().cloned().collect();
        for addr in remaining_fn_addrs {
            let mut fn_entry = self.new_entry(root, DW_TAG_subprogram);
            fn_entry.init_fn(addr, &mut fn_map, type_map);
        }
    }

    /// Writes the anvill data as DWARF debug info and updates the type map with
    /// new type entries.
    pub fn process_anvill(&mut self, mut anvill: AnvillData, type_map: &mut TypeMap) {
        let AnvillData {
            types,
            mut var_map,
            mut fn_map,
        } = anvill;
        self.update_types(types, type_map);

        self.for_each_entry(|dwarf, &entry_id| {
            let entry = dwarf.get(entry_id);
            match entry.tag() {
                constants::DW_TAG_variable => {
                    let mut var_entry = dwarf.entry_ref(entry_id);
                    var_entry.update_var(&mut var_map, type_map);
                },
                constants::DW_TAG_subprogram => {
                    let mut fn_entry = dwarf.entry_ref(entry_id);
                    fn_entry.update_anvill_fn(&mut fn_map, type_map);
                },
                _ => (),
            }
        });

        let root = self.root();
        let remaining_fn_addrs: Vec<_> = fn_map.keys().cloned().collect();
        for addr in remaining_fn_addrs {
            let mut fn_entry = self.new_entry(root, DW_TAG_subprogram);
            fn_entry.init_anvill_fn(addr, &mut fn_map, type_map);
        }

        let remaining_var_addrs: Vec<_> = var_map.keys().cloned().collect();
        for addr in remaining_var_addrs {
            let mut var_entry = self.new_entry(root, DW_TAG_variable);
            var_entry.init_var(addr, &mut var_map, type_map);
        }
        assert!(fn_map.is_empty());
    }

    /// Writes the STR BSI data as DWARF debug info and updates the type map
    /// with new type entries.
    pub fn process_str_bsi(&mut self, mut str_bsi: StrBsiData, type_map: &mut TypeMap) {
        let StrBsiData { types, mut fn_map } = str_bsi;
        self.update_types(types, type_map);

        self.for_each_entry(|dwarf, &entry_id| {
            let entry = dwarf.get(entry_id);
            if let constants::DW_TAG_subprogram = entry.tag() {
                let mut fn_entry = dwarf.entry_ref(entry_id);
                fn_entry.update_str_fn(&mut fn_map, type_map);
            };
        });

        let root = self.root();
        let remaining_fn_addrs: Vec<_> = fn_map.keys().cloned().collect();
        for addr in remaining_fn_addrs {
            let mut fn_entry = self.new_entry(root, DW_TAG_subprogram);
            fn_entry.init_str_fn(addr, &mut fn_map, type_map);
        }
    }
}
