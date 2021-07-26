use anvill::AnvillHints;
use anyhow::Result;
//use dwarf_unit::process_dwarf_units;
use elf::ELF;
use std::path::PathBuf;
use structopt::StructOpt;

mod anvill;
mod dwarf_attr;
mod dwarf_die;
mod dwarf_unit;
mod elf;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(short = "b", long = "bin_in", parse(from_os_str))]
    binary_path: PathBuf,
    #[structopt(short = "a", long = "anvill", parse(from_os_str))]
    anvill_path: Option<PathBuf>,
    #[structopt(short = "m", long = "mindsight", parse(from_os_str))]
    mindsight_path: Option<PathBuf>,
}

//pub type TypeMap<'a, 'b> = HashMap<&'a str, EntryRef<'b>>;

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let binary_path = opt.binary_path;
    let anvill_path = opt.anvill_path;
    let anvill_hints = if let Some(path) = anvill_path {
        Some(AnvillHints::new(path)?)
    } else {
        None
    };
    //let anvill_hints = anvill_path.map(|path| AnvillHints::new(path));
    // The `update_fn` pass will remove entries from this map then the `create_fn`
    // will create DIEs for the remaining entries
    let (anvill_fn_map, anvill_types) = if let Some(hint) = anvill_hints.as_ref() {
        (Some(hint.functions()), Some(hint.types()))
    } else {
        (None, None)
    };

    let mut elf = ELF::new(&binary_path)?;

    let updated_sections = elf.sections()?;

    ELF::dump_sections(&updated_sections)?;

    Ok(())
}
