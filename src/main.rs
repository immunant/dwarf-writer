use anvill_parser::{AnvillHints, Arch, X86};
use anyhow::Result;
use dwarf_writer::ELF;
use gimli::constants::*;
use gimli::write;
use gimli::write::{DebuggingInformationEntry, EndianVec, Sections, StringTable, UnitEntryId};
use gimli::RunTimeEndian;
use std::env::{args, Args};
use std::io::Write;
use std::str::from_utf8;
use std::{fmt, fs, io};

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

fn open_hints(path: &str) -> Result<AnvillHints<X86>> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let hints = serde_json::from_reader(reader)?;
    Ok(hints)
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

fn attr_to_str<'a>(attr: &'a write::AttributeValue, strings: &'a StringTable) -> Option<&'a [u8]> {
    // TODO: This is missing some cases
    match attr {
        write::AttributeValue::String(s) => Some(s),
        write::AttributeValue::StringRef(str_id) => Some(strings.get(*str_id)),
        _ => None,
    }
}

// References to a subset of `gimli::write::Dwarf` to modify a specific DIE.
struct DIERef<'a> {
    // The unit containing the DIE
    unit: &'a mut write::Unit,
    // The DIE's ID
    id: UnitEntryId,

    // Miscellaneous DWARF info for debugging
    strings: &'a write::StringTable,
}

// TODO: Make another pass that creates subprogram DIEs and adds a function
// prototype. Maybe this pass should return the anvill fn data used to make it
// easier to track what anvill fn data should be used in the create_subprogram
// pass
/// Updates or creates a function prototype for an existing DW_TAG_subprogram
/// DIE.
fn update_fn_prototype<A: Arch>(die_ref: DIERef, hints: &AnvillHints<A>) -> Result<()> {
    let DIERef { unit, id, strings } = die_ref;
    let die = unit.get_mut(id);

    // Get this function's name from the existing DWARF data
    let name_attr = die.get(DW_AT_name).expect("");
    let name = from_utf8(attr_to_str(name_attr, strings).expect(""))?;

    // Get the anvill data for this function
    // TODO: Create the `functions` hashmap in the constructor
    let all_anvill_data = hints.functions();
    let anvill_fn_data = all_anvill_data.get(name);

    // Only modify DIE if anvill function parameter data is present
    if let Some(fn_data) = anvill_fn_data {
        if let Some(params) = fn_data.parameters() {
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
    Ok(())
}

fn main() -> Result<()> {
    let (hints_path, binary_path) = match parse_args(&mut args()) {
        Ok(paths) => paths,
        Err(e) => {
            print_help();
            return Err(e)
        },
    };
    let hints = open_hints(&hints_path)?;

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

                    // collect grandchildren for later processing before mutating the DIE
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
                        update_fn_prototype(die_ref, &hints)?;
                    }
                }
            }
        }
        Ok(())
    })?;

    write_sections(&updated_sections)
}
