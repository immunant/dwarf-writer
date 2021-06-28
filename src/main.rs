use anvill_parser::{AnvillFnMap, AnvillHints, Arch};
use anyhow::Result;
use dwarf_writer::ELF;
use gimli::constants::*;
use gimli::write;
use gimli::write::{EndianVec, Sections, StringTable, UnitEntryId};
use gimli::RunTimeEndian;
use std::env::{args, Args};
use std::fs;
use std::io::Write;
use std::str::from_utf8;

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

fn name_to_str<'a>(attr: &'a write::AttributeValue, strings: &'a StringTable) -> Option<&'a [u8]> {
    // TODO: This is missing some cases
    match attr {
        write::AttributeValue::String(s) => Some(s),
        write::AttributeValue::StringRef(str_id) => Some(strings.get(*str_id)),
        _ => None,
    }
}

fn low_pc_to_u64(attr: &write::AttributeValue) -> Option<u64> {
    // TODO: Handle Address::Symbol
    match attr {
        write::AttributeValue::Address(write::Address::Constant(addr)) => Some(*addr),
        _ => None,
    }
}

fn high_pc_to_u64(attr: &write::AttributeValue) -> Option<u64> {
    match attr {
        write::AttributeValue::Address(write::Address::Constant(addr)) => Some(*addr),
        write::AttributeValue::Udata(addr) => Some(*addr),
        _ => None,
    }
}

// References to a subset of `gimli::write::Dwarf` to modify a specific DIE.
struct DIERef<'a> {
    // The unit containing the DIE
    unit: &'a mut write::Unit,
    // The DIE's ID
    id: UnitEntryId,

    // Miscellaneous DWARF info
    strings: &'a write::StringTable,
}

/// Initializes a newly created subprogram DIE.
fn create_fn<A: Arch>(die_ref: DIERef, addr: u64, anvill_data: &mut AnvillFnMap<A>) {
    let die = die_ref.unit.get_mut(die_ref.id);
    die.set(
        DW_AT_low_pc,
        write::AttributeValue::Address(write::Address::Constant(addr)),
    );
    update_fn(die_ref, anvill_data)
}
// TODO: Make another pass that creates subprogram DIEs and adds a function
// prototype. Maybe this pass should return the anvill fn data used to make it
// easier to track what anvill fn data should be used in the create_subprogram
// pass
/// Updates or creates a function prototype for an existing DW_TAG_subprogram
/// DIE.
fn update_fn<A: Arch>(die_ref: DIERef, anvill_data: &mut AnvillFnMap<A>) {
    let DIERef { unit, id, strings } = die_ref;
    let die = unit.get_mut(id);

    // Get this function's address from the existing DWARF data
    let low_pc_attr = die.get(DW_AT_low_pc).expect("");
    let low_pc = low_pc_to_u64(low_pc_attr).expect("");

    //let high_pc_attr = die.get(DW_AT_high_pc).expect("");
    //let high_pc = high_pc_to_u64(high_pc_attr).expect("");

    // Get this function's name from the existing DWARF data
    //let name_attr = die.get(DW_AT_name).expect("");
    //let name = from_utf8(name_to_str(name_attr, strings).expect(""))?;

    // Get the anvill data for this function
    let fn_data = anvill_data.remove(&low_pc);

    // Only modify DIE if anvill function parameter data is present
    if let Some(fn_data) = fn_data {
        if let Some(params) = fn_data.0.parameters() {
            // TODO: This overwrites DW_AT_prototyped = false which shouldn't be valid
            // anyway?
            die.set(DW_AT_prototyped, write::AttributeValue::Flag(true));
            for param in params {
                // TODO: Check that the argument doesn't already exist, i.e. no
                // DW_TAG_formal_parameter DIE should have a matching DW_AT_name or
                // DW_AT_location

                let param_id = unit.add(id, DW_TAG_formal_parameter);
                let param_die = unit.get_mut(param_id);
                if let Some(param_name) = param.name() {
                    param_die.set(
                        DW_AT_name,
                        write::AttributeValue::String(param_name.as_bytes().to_vec()),
                    );
                }
                // TODO: Set DW_AT_type
                // TODO: Set DW_AT_location
            }
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

    let updated_sections = elf.with_dwarf_mut(|dwarf| {
        for idx in 0..dwarf.units.count() {
            let unit_id = dwarf.units.id(idx);
            let unit = dwarf.units.get_mut(unit_id);

            // process root DIE
            let root_die = unit.get(unit.root());
            println!("Processing root DIE {:?}", root_die.tag().static_string());

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
                            id: die_id,
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
                    id: fn_id,
                    strings: &dwarf.strings,
                };
                create_fn(die_ref, addr, &mut fn_map);
            }
        }
        Ok(())
    })?;

    write_sections(&updated_sections)
}
