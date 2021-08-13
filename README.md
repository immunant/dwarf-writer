# Dwarf-writer

Dwarf-writer updates a program's DWARF debug sections with information obtained through disassembly. The target program's debug info can either be updated in-place or written to a copy of the program. Writing the updated debug sections to individual files is also supported. For details on the sources of disassembly data and supported target architectures see [Disassembly_data.md](Disassembly_data.md).

## Building and prerequisites

Building dwarf-writer requires a [rust installation](https://www.rust-lang.org/) and [objcopy](https://www.gnu.org/software/binutils/) cross-compiled for the target program's architecture. Most linux distributions provide a version of objcopy that can be used for native binaries and is used by default if it's in the system's PATH. After setting up those prerequisites, clone this tool's repo and build it with the following steps.

```
$ git clone https://github.com/immunant/dwarf-writer

$ cd dwarf-writer

$ cargo build --release

# To run dwarf-writer with cargo
$ cargo run -- -a $ANVILL_JSON -x /path/to/objcopy $BINARY

# To run dwarf-writer without cargo
$ ./target/release/dwarf-writer -a $ANVILL_JSON -x /path/to/objcopy $BINARY

# To install dwarf-writer
$ cargo install --path .
```

## Usage

```
$ dwarf-writer -h
dwarf-writer 0.1.0

USAGE:
    dwarf-writer [FLAGS] [OPTIONS] <input> [output]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    

OPTIONS:
    -a, --anvill <anvill-data>          Anvill disassembly data
    -l, --logging <level>               Set logging level explicitly
    -x, --objcopy <objcopy-path>        Alternate objcopy to use (defaults to objcopy in PATH)
    -s, --section-files <output-dir>    Output directory for writing DWARF sections to individual files

ARGS:
    <input>     Input binary
    <output>    Output binary

# To update the program's debug info in-place using the objcopy in PATH
$ dwarf-writer -a $ANVILL_JSON $BINARY

# To update the debug info in a copy of the program
$ dwarf-writer -a $ANVILL_JSON $IN_BINARY $OUT_BINARY

# Specify an alternate path to objcopy to run dwarf-writer on binaries for other architectures
$ dwarf-writer -a $ANVILL_JSON -x /usr/bin/arm-linux-gnueabihf-objcopy $BINARY

# To view the program's updated debug info
$ llvm-dwarfdump $BINARY
```
