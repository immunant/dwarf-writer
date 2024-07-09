# Dwarf-writer

Dwarf-writer updates a program's DWARF debug sections and ELF symbols with information obtained through disassembly. The target program can either be updated in-place or written to a copy of the program. Writing the updated debug sections to individual files is also supported. For details on the sources of disassembly data and supported target architectures see [Disassembly_data.md](Disassembly_data.md).
![demo](demo.gif)

## Building and prerequisites

Building dwarf-writer requires a [rust installation](https://www.rust-lang.org/) and [objcopy](https://www.gnu.org/software/binutils/) cross-compiled for the target program's architecture. Most linux distributions provide a version of objcopy that can be used for native binaries and is used by default if it's in the system's PATH. After setting up those prerequisites, clone this tool's repo and build it with the following steps.

```
$ git clone https://github.com/immunant/dwarf-writer

$ cd dwarf-writer

$ cargo build --release

# To run dwarf-writer with cargo
$ cargo run -- -a $ANVILL_JSON -b $STR_JSON -x /path/to/objcopy $BINARY

# To run dwarf-writer without cargo
$ ./target/release/dwarf-writer -a $ANVILL_JSON -b $STR_JSON -x /path/to/objcopy $BINARY

# To install dwarf-writer
$ cargo install --path .
```

## Usage

```
$ dwarf-writer -h

USAGE:
    dwarf-writer [OPTIONS] <input> [output]

ARGS:
    <input>     Input binary
    <output>    Output binary

OPTIONS:
    -a, --anvill <anvill-data>          Anvill disassembly data
    -b, --str-bsi <str-data>            STR BSI disassembly data
    -g, --ghidra <ghidra>               Ghidra disassembly data
    -h, --help                          Print help information
    -l, --logging <level>               Set logging level explicitly
        --omit-functions                Avoid emitting DW_TAG_subprogram entries
        --omit-symbols                  Avoid adding ELF symbols
        --omit-variables                Avoid emitting DW_TAG_variable entries for Anvill
    -s, --section-files <output-dir>    Output directory for writing DWARF sections to individual
                                        files
    -u, --use-all-str                   Use all entries in STR data regardless of confidence level
    -v, --verbose
    -x, --objcopy <objcopy-path>        Alternate objcopy to use (defaults to objcopy in PATH)


# To update the program's debug info in-place using the objcopy in PATH
$ dwarf-writer -a $ANVILL_JSON -b $STR_JSON $BINARY

# To update the debug info in a copy of the program
$ dwarf-writer -a $ANVILL_JSON -b $STR_JSON $IN_BINARY $OUT_BINARY

# Specify an alternate path to objcopy to run dwarf-writer on binaries for other architectures
$ dwarf-writer -a $ANVILL_JSON -b $STR_JSON -x /usr/bin/arm-linux-gnueabihf-objcopy $BINARY

# To view the program's updated debug info
$ llvm-dwarfdump $BINARY
```

# Acknowledgements

This material is based upon work supported by the Defense Advanced Research Projects Agency (DARPA) and Naval Information Warfare Center Pacific (NIWC Pacific) under Contract Number N66001-20-C-4027 and 140D0423C0063.

Any opinions, findings and conclusions or recommendations expressed in this material are those of the author(s) and do not necessarily reflect the views of the Defense Advanced Research Projects Agency (DARPA), NIWC Pacific, or its Contracting Agent, the U.S. Department of the Interior, Interior Business Center, Acquisition Services Directorate, Division III.
