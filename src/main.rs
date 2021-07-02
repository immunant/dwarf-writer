use anvill_parser::{AnvillFnMap, AnvillHints};
use anyhow::Result;
use dwarf_writer::ELF;
use gimli::constants::*;
use gimli::write;
use gimli::write::{EndianVec, LineProgram, Sections, StringTable, Unit, UnitEntryId};
use gimli::{Encoding, Format, RunTimeEndian};
use object::Object;
use std::env::{args, Args};
use std::fs;
use std::io::Write;

mod anvill_parser;
mod dwarf_writer;

fn print_help() {
    println!("help message goes here");
}

fn parse_args(args: &mut Args) -> Result<(String, String)> {
    let hints = args
        .skip(1)
        .next()
        .ok_or(anyhow::Error::msg("Missing hints file"))?;
    let binary = args
        .next()
        .ok_or(anyhow::Error::msg("Missing binary file"))?;
    Ok((hints, binary))
}

fn write_sections(sections: &Sections<EndianVec<RunTimeEndian>>) -> Result<()> {
    sections.for_each(|id, data| {
        if !data.slice().is_empty() {
            println!("Writing {} section", id.name());
            let file_name = &id.name()[1..];
            let mut file = fs::File::create(file_name)?;
            file.write_all(data.slice())?;
        }
        Ok(())
    })
}

fn name_to_str<'a>(attr: &'a write::AttributeValue, strings: &'a StringTable) -> &'a [u8] {
    // TODO: This is missing some cases
    match attr {
        write::AttributeValue::String(s) => s,
        write::AttributeValue::StringRef(str_id) => strings.get(*str_id),
        _ => panic!("Unhandled `AttributeValue` variant in `name_to_str`"),
    }
}

fn low_pc_to_u64(attr: &write::AttributeValue) -> u64 {
    // TODO: Handle Address::Symbol
    match attr {
        write::AttributeValue::Address(write::Address::Constant(addr)) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `low_pc_to_u64`"),
    }
}

fn high_pc_to_u64(attr: &write::AttributeValue) -> u64 {
    match attr {
        write::AttributeValue::Address(write::Address::Constant(addr)) => *addr,
        write::AttributeValue::Udata(addr) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `high_pc_to_u64`"),
    }
}

// References to a subset of `gimli::write::Dwarf` to modify a specific DIE.
struct DIERef<'a> {
    // The unit containing the DIE
    unit: &'a mut write::Unit,
    // The DIE's ID
    self_id: UnitEntryId,

    // Miscellaneous DWARF info
    strings: &'a write::StringTable,
}

/// Initializes a newly created subprogram DIE.
fn create_fn(die_ref: DIERef, addr: u64, anvill_data: &mut AnvillFnMap) {
    let die = die_ref.unit.get_mut(die_ref.self_id);
    die.set(
        DW_AT_low_pc,
        write::AttributeValue::Address(write::Address::Constant(addr)),
    );
    update_fn(die_ref, anvill_data)
}

/// Updates or creates a function prototype for an existing DW_TAG_subprogram
/// DIE.
fn update_fn(die_ref: DIERef, anvill_data: &mut AnvillFnMap) {
    let DIERef {
        unit,
        self_id,
        strings,
    } = die_ref;
    let die = unit.get(self_id);

    // Get this function's address from the existing DWARF data
    let low_pc_attr = die
        .get(DW_AT_low_pc)
        .expect("No DW_AT_low_pc found in DW_TAG_subprogram DIE");
    let low_pc = low_pc_to_u64(low_pc_attr);

    // Get the anvill data for this function
    let fn_data = anvill_data.remove(&low_pc);
    if let Some(fn_data) = fn_data {
        // Update function name overwriting any existing DW_AT_name
        if let Some(name) = fn_data.name {
            let die = unit.get_mut(self_id);
            die.set(
                DW_AT_name,
                write::AttributeValue::String(name.as_bytes().to_vec()),
            );
        }
        // Update function parameters
        if let Some(parameters) = fn_data.func.parameters() {
            for param in parameters {
                println!("anvill had param {:?} for {:#x}", param, low_pc);
                // Search for a matching DIE by name
                // TODO: Search for a matching formal parameter DIE by location
                let die = unit.get(self_id);
                let matching_die_id = die.children().find(|&&child_id| {
                    let child_die = unit.get(child_id);
                    let child_tag = child_die.tag();
                    let name_attr = child_die
                        .get(DW_AT_name)
                        .expect("assume anvill json always names args for now");
                    let name = name_to_str(name_attr, strings);
                    child_tag == DW_TAG_formal_parameter && name == param.name().unwrap().as_bytes()
                });
                // Add a formal parameter DIE if a matching DIE wasn't found
                let param_id = match matching_die_id {
                    Some(&id) => id,
                    None => unit.add(self_id, DW_TAG_formal_parameter),
                };
                let param_die = unit.get_mut(param_id);
                if let Some(param_name) = param.name() {
                    param_die.set(
                        DW_AT_name,
                        write::AttributeValue::String(param_name.as_bytes().to_vec()),
                    );
                    //param_die.set(DW_AT_base_type,
                }
            }
            // Mark the subprogram DIE as prototyped
            let die = unit.get_mut(self_id);
            die.set(DW_AT_prototyped, write::AttributeValue::Flag(true));
        }
    }
}

fn main() -> Result<()> {
    let (hints_path, binary_path) = match parse_args(&mut args()) {
        Ok(paths) => paths,
        Err(e) => {
            print_help();
            return Err(e)
        },
    };
    let hints = AnvillHints::new(&hints_path)?;
    // The `update_fn` pass will remove entries from this map then the `create_fn`
    // will create DIEs for the remaining entries
    let mut fn_map = hints.functions();

    let mut elf = ELF::new(&binary_path)?;

    let updated_sections = elf.with_dwarf_mut(|elf, dwarf| {
        if dwarf.units.count() == 0 {
            let format = if elf.is_64() {
                Format::Dwarf64
            } else {
                Format::Dwarf32
            };
            let encoding = Encoding {
                address_size: format.word_size(),
                format,
                // TODO: Make this configurable
                version: 4,
            };
            let line_program = LineProgram::none();
            let root = Unit::new(encoding, line_program);
            dwarf.units.add(root);
        };
        // TODO: How should DWARF version be handled here?
        for idx in 0..dwarf.units.count() {
            let unit_id = dwarf.units.id(idx);
            let unit = dwarf.units.get_mut(unit_id);

            // process root DIE
            let root_die = unit.get(unit.root());
            println!("Processing root DIE {:?}", root_die.tag().static_string());
            // TODO: Add/check for type DIEs. This means iterating over functions/variables
            // to add all types that don't already exist
            println!("{:#?}", hints.types());

            let mut children = root_die.children().cloned().collect::<Vec<_>>();
            while !children.is_empty() {
                for die_id in children.drain(..).collect::<Vec<_>>() {
                    let die = unit.get(die_id);

                    // collect grandchildren processing before mutating the DIE since newly created
                    // DIEs should not be processed
                    let mut grandchildren = die.children().cloned().collect();
                    children.append(&mut grandchildren);

                    // process DIE
                    println!("Processing DIE {:?}", die.tag().static_string());
                    if die.tag() == DW_TAG_subprogram {
                        let die_ref = DIERef {
                            unit,
                            self_id: die_id,
                            strings: &dwarf.strings,
                        };
                        update_fn(die_ref, &mut fn_map);
                    }
                }
            }
            // Add a subprogram DIE for each remaining `fn_map` entry
            let remaining_entries = fn_map.keys().cloned().collect::<Vec<_>>();
            for addr in remaining_entries {
                let fn_id = unit.add(unit.root(), DW_TAG_subprogram);
                let die_ref = DIERef {
                    unit,
                    self_id: fn_id,
                    strings: &dwarf.strings,
                };
                create_fn(die_ref, addr, &mut fn_map);
            }
        }
        Ok(())
    })?;

    write_sections(&updated_sections)
}
