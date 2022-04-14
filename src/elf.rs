use crate::into_gimli::IntoGimli;
use crate::symbols::Symbols;
use anyhow::Result;
use gimli::read;
use gimli::write::{Address, Dwarf, EndianVec, Sections};
use gimli::{EndianSlice, RunTimeEndian, SectionId};
use log::warn;
use object::{Object, ObjectSection, ObjectSymbol};
use std::borrow::Cow;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

/// An ELF and its DWARF debug data.
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct ELF {
    /// The initial data read from the ELF file. This buffer is not kept in sync
    /// with the DWARF data written through the `dwarf` field so it should only
    /// be used to read the ELF object data.
    initial_buffer: Vec<u8>,
    /// Mutable DWARF debug data.
    pub dwarf: Dwarf,
    elf_path: PathBuf,
}

impl ELF {
    /// Creates a new `ELF` from an input file path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path.as_ref())?;
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
        let owned_dwarf = read::Dwarf::load(load_section)?;
        let read_only_dwarf = owned_dwarf.borrow(|section| EndianSlice::new(section, endianness));
        let dwarf = Dwarf::from(&read_only_dwarf, &|addr| Some(Address::Constant(addr)))?;

        Ok(Self {
            initial_buffer: buffer,
            dwarf,
            elf_path: path.as_ref().to_path_buf(),
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

    pub fn update_binary(
        mut self, output_path: Option<PathBuf>, objcopy_path: Option<PathBuf>,
        output_dir: Option<PathBuf>, syms: Symbols,
    ) -> Result<()> {
        let temp_dir = tempdir()?;
        let dir = match output_dir {
            Some(ref dir) => dir.as_path(),
            None => temp_dir.path(),
        };
        let output_path = match output_path {
            Some(path) => {
                let mut output_file = fs::File::create(&path)?;
                output_file.write_all(&self.initial_buffer)?;
                path
            },
            None => self.elf_path.clone(),
        };
        let objcopy = &objcopy_path.unwrap_or_else(|| "objcopy".into());

        // Update symbols
        let mut cmd = Command::new(objcopy);
        let object = self.object();
        let existing_syms: Vec<_> = object
            .symbols()
            .map(|existing| (existing.name(), existing.address()))
            .collect();
        for s in syms.0 {
            // If an existing symbol has a matching address, find its name
            let addr_exists = existing_syms.iter().find_map(|&(name, addr)| {
                if addr == s.value {
                    name.map(|n| Some(n)).unwrap_or(None)
                } else {
                    None
                }
            });
            // If an existing symbol has a matching name, find its address
            let name_exists = existing_syms.iter().find_map(|&(name, addr)| {
                name.map(|n| if n == s.name { Some(addr) } else { None })
                    .unwrap_or(None)
            });
            match (addr_exists, name_exists) {
                (None, None) => {
                    // Add a new symbol if no existing symbol has a matching address or name
                    cmd.arg("--add-symbol").arg(s.objcopy_add_cmd());
                },
                (Some(old_name), None) => {
                    // If a symbol with the same address has a different name, update its name
                    cmd.arg("--redefine-sym")
                        .arg(format!("{}={}", old_name, s.name));
                },
                (None, Some(_)) => {
                    // If a symbol with the same name has a different address, update its address by
                    // first stripping the existing symbol then adding it again
                    cmd.arg("--strip-symbol").arg(s.name.to_owned());
                    cmd.arg("--add-symbol").arg(s.objcopy_add_cmd());
                },
                (Some(existing_name), Some(existing_addr)) => {
                    // If an existing symbol has the same address and name we don't need to update
                    // anything.
                    assert!(existing_name == s.name);
                    assert!(existing_addr == s.value);
                },
            };
        }
        let output = cmd.arg(output_path.as_path()).output()?;
        let stdout = std::str::from_utf8(&output.stdout)?;
        let stderr = std::str::from_utf8(&output.stderr)?;
        if !stdout.is_empty() {
            warn!("{}", stdout);
        }
        if !stderr.is_empty() {
            warn!("{}", stderr);
        }

        // Update DWARF info
        let updated_sections = &self.sections()?;

        updated_sections.for_each(|section, data| {
            if !data.slice().is_empty() {
                // Remove leading '.' in section name to avoid creating dot files
                let file_name = &section.name()[1..];
                let section_path = &dir.join(file_name);
                // Write section data to a file
                let mut file = fs::File::create(section_path)?;
                file.write_all(data.slice())?;

                // Pass section file and binary through objcopy
                let section_exists = self
                    .object()
                    .sections()
                    .any(|s| s.name() == Ok(section.name()));
                let objcopy_cmd = if section_exists {
                    "--update-section"
                } else {
                    "--add-section"
                };

                let mut objcopy_arg = section.name().to_string();
                objcopy_arg.push('=');
                objcopy_arg.push_str(section_path.as_path().to_str().unwrap());

                let output = Command::new(objcopy)
                    .arg(objcopy_cmd)
                    .arg(objcopy_arg.as_str())
                    .arg(output_path.as_path())
                    .output()?;
                let stdout = std::str::from_utf8(&output.stdout)?;
                let stderr = std::str::from_utf8(&output.stderr)?;
                if !stdout.is_empty() {
                    warn!("{}", stdout);
                }
                if !stderr.is_empty() {
                    warn!("{}", stderr);
                }
            }
            Ok(())
        })
    }
}
