use anyhow::Result;
use gimli::write;
use gimli::write::{EndianVec, Sections};
use gimli::{Dwarf, EndianSlice, RunTimeEndian, SectionId};
use object::{Object, ObjectSection};
use std::borrow::Cow;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug)]
pub struct ELF {
    buffer: Vec<u8>,
}

impl ELF {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        Ok(ELF { buffer })
    }

    fn object(&self) -> Result<object::File> {
        Ok(object::File::parse(self.buffer.as_slice())?)
    }

    /// Calls the specified closure with immutable access to the DWARF sections
    /// provided by `gimli::read::Dwarf`
    pub fn with_dwarf<F, R>(&self, mut f: F) -> Result<R>
    where F: FnMut(&object::File, &Dwarf<EndianSlice<RunTimeEndian>>) -> Result<R> {
        let obj = self.object()?;
        let endianness = obj.endianness().into_gimli();

        let load_section = |id: SectionId| -> Result<Cow<[u8]>> {
            let empty: Cow<[u8]> = Cow::Borrowed(&[][..]);
            Ok(obj
                .section_by_name(id.name())
                .map(|ref section| {
                    section
                        .uncompressed_data()
                        .expect("Could not decompress section data")
                })
                .unwrap_or(empty))
        };

        let dwarf_cow = Dwarf::load(load_section)?;

        let dwarf = dwarf_cow.borrow(|section| EndianSlice::new(&section, endianness));
        Ok(f(&obj, &dwarf)?)
    }

    // TODO: Returning `Result<R>` here and having a separate method for returning
    // the sections might be clearer
    /// Calls the specified closure with mutable access to the DWARF sections
    /// provided by `gimli::write::Dwarf`
    pub fn with_dwarf_mut<F>(&mut self, mut f: F) -> Result<Sections<EndianVec<RunTimeEndian>>>
    where F: FnMut(&object::File, &mut write::Dwarf) -> Result<()> {
        let obj = self.object()?;
        let endianness = obj.endianness().into_gimli();

        self.with_dwarf(|obj, dwarf| {
            let mut dwarf =
                write::Dwarf::from(&dwarf, &|addr| Some(write::Address::Constant(addr)))?;
            f(obj, &mut dwarf)?;
            let mut sections = Sections::new(EndianVec::new(endianness));
            dwarf.write(&mut sections)?;
            Ok(sections)
        })
    }

    pub fn write_sections(sections: &Sections<EndianVec<RunTimeEndian>>) -> Result<()> {
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
}

/// Generic trait for converting to gimli-specific types.
pub trait IntoGimli<T> {
    fn into_gimli(self) -> T;
}

impl IntoGimli<gimli::RunTimeEndian> for object::endian::Endianness {
    fn into_gimli(self) -> gimli::RunTimeEndian {
        use gimli::RunTimeEndian as gimli;
        use object::endian::Endianness as obj;
        match self {
            obj::Little => gimli::Little,
            obj::Big => gimli::Big,
        }
    }
}
