use crate::into_gimli::IntoGimli;
use anyhow::Result;
use gimli::write;
use gimli::write::{EndianVec, Sections};
use gimli::{Dwarf, EndianSlice, RunTimeEndian, SectionId};
use object::{Object, ObjectSection};
use std::borrow::Cow;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// An ELF and its DWARF debug data.
#[derive(Debug)]
pub struct ELF {
    /// The initial data read from the ELF file. This buffer is not kept in sync
    /// with the DWARF data written through the `dwarf` field so it should only
    /// be used to read the ELF object data.
    initial_buffer: Vec<u8>,
    /// Mutable DWARF debug data.
    pub dwarf: write::Dwarf,
}

impl ELF {
    /// Creates a new `ELF` from an input file path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let obj = object::File::parse(buffer.as_slice())?;
        let endianness = obj.endianness().into_gimli();

        // Specify how to load an ELF section
        let load_section = |id: SectionId| -> Result<Cow<[u8]>> {
            let empty = Cow::Borrowed(&[][..]);
            let section = obj.section_by_name(id.name()).map(|ref section| {
                section
                    .uncompressed_data()
                    .expect("Could not decompress section data")
            });
            Ok(section.unwrap_or(empty))
        };
        let owned_dwarf = Dwarf::load(load_section)?;
        let read_only_dwarf = owned_dwarf.borrow(|section| EndianSlice::new(&section, endianness));
        let dwarf = write::Dwarf::from(&read_only_dwarf, &|addr| {
            Some(write::Address::Constant(addr))
        })?;

        Ok(Self {
            initial_buffer: buffer,
            dwarf,
        })
    }

    /// Parses the ELF object data. Note this object data is not kept
    /// synchronized with changes to DWARF debug data.
    pub fn object(&self) -> object::File {
        // The constructor ensures that the buffer is a valid object file
        object::File::parse(self.initial_buffer.as_slice()).unwrap()
    }

    /// Write the DWARF debug data to ELF sections.
    pub fn sections(&mut self) -> Result<Sections<EndianVec<RunTimeEndian>>> {
        let endianness = self.object().endianness().into_gimli();
        let mut sections = Sections::new(EndianVec::new(endianness));
        self.dwarf.write(&mut sections)?;
        Ok(sections)
    }

    /// Dump the specified ELF sections to individual files.
    pub fn dump_sections(sections: &Sections<EndianVec<RunTimeEndian>>) -> Result<()> {
        sections.for_each(|id, data| {
            if !data.slice().is_empty() {
                let file_name = &id.name()[1..];
                let mut file = fs::File::create(file_name)?;
                file.write_all(data.slice())?;
            }
            Ok(())
        })
    }
}
