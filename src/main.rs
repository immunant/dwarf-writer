use crate::anvill_parser::X86;
use anvill_parser::AnvillHints;
use anyhow::Result;
use dwarf_writer::ELF;
use gimli::constants::{DW_AT_name, DW_TAG_subprogram};
use gimli::write;
use gimli::write::{EndianVec, Sections, StringTable};
use gimli::RunTimeEndian;
use std::env::{args, Args};
use std::io::Write;
use std::{fmt, fs, io};

mod anvill_parser;
mod dwarf_writer;

/// Defines information that may be provided by the input disassembly data.
pub trait Hints: fmt::Debug {
    /// Returns the names of symbols known to be functions.
    fn fn_names(&self) -> Option<Vec<&String>>;
}

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
            let file_name = &id.name()[1..];
            let mut file = fs::File::create(file_name)?;
            file.write_all(data.slice())?;
        }
        Ok(())
    })
}

fn attr_to_str<'a>(attr: &'a write::AttributeValue, strings: &'a StringTable) -> Option<Vec<u8>> {
    // TODO: This is missing some cases
    match attr {
        write::AttributeValue::String(s) => Some(s.clone()),
        write::AttributeValue::StringRef(str_id) => Some(strings.get(*str_id).to_vec()),
        _ => None,
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
    let hints = open_hints(&hints_path)?;
    let fn_names = hints
        .fn_names()
        .expect("Anvill input file did not contain functions and symbols");

    let mut elf = ELF::new(&binary_path)?;

    let updated_sections = elf.with_dwarf_mut(|dwarf| {
        for idx in 0..dwarf.units.count() {
            let unit_id = dwarf.units.id(idx);
            let unit = dwarf.units.get_mut(unit_id);

            let root_die = unit.get(unit.root());

            // TODO: This doesn't handle grandchildren, etc.
            let children: Vec<_> = root_die.children().cloned().collect();
            for unit_entry_id in children {
                let die = unit.get_mut(unit_entry_id);
                // Consider only function DIEs
                if die.tag() == DW_TAG_subprogram {
                    let fn_name = die
                        .get(DW_AT_name)
                        .unwrap_or_else(|| todo!("Can a DW_TAG_subprogram have no name?"));
                    let fn_name = attr_to_str(fn_name, &dwarf.strings)
                        .expect("DW_AT_name should be a string");
                    let matching_name = fn_names.iter().find(|name| name.as_bytes() == fn_name);
                    matching_name.map(|_name| {
                        die.set(
                            DW_AT_name,
                            write::AttributeValue::String("gg".as_bytes().to_vec()),
                        );
                    });
                }
            }
        }
        Ok(())
    })?;

    write_sections(&updated_sections)
}
