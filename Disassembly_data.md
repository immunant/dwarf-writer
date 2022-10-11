# Disassembly data sources

## Anvill's python plugin

This tool supports a subset of the [JSON specification](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md) produced by [Anvill's](https://github.com/lifting-bits/anvill/) python plugin which supports x86, ARM and SPARC. See the spec docs for details on what it can provide.

### Setting it up

Only the python plugin in anvill is required to produce disassembly data. This plugin requires either [Binary Ninja](https://docs.binary.ninja/getting-started.html) or [IDA PRO](https://hex-rays.com/ida-pro/) which are used as backends. After installing one of these disassemblers install the necessary parts of anvill with the following steps.

```
$ git clone https://github.com/lifting-bits/anvill

$ python3 -m pip install anvill/
```

### Basic usage
```
$ python3 -m anvill --bin_in $INPUT_BINARY --spec_out $OUTPUT_JSON
$ dwarf-wrter -a $OUTPUT_JSON $BINARY
```

### Capabilities

This tool supports creating and updating DWARF entries for functions and global variables with this data. Only the following attributes are currently supported.

- DW_TAG_variable (global variables)
    - DW_AT_location
    - DW_AT_name
    - DW_AT_type
- DW_TAG_subprogram (functions)
    - DW_AT_low_pc
    - DW_AT_name
    - DW_AT_return_addr
    - DW_AT_noreturn
    - DW_AT_prototyped
    - DW_AT_type
    - DW_TAG_formal_parameter (arguments)
        - DW_AT_location
        - DW_AT_name
        - DW_AT_type

There is also experimental support for adding new ELF symbols for functions and variables. If an existing symbol has an address (name) matching a symbol in the input file, the symbol's name (address) is updated. There is currently no support for specifying symbol sections (defaults to ABS).

## STR BSI format

The tool also supports another JSON format that matches disassembled functions with their source.

```
$ dwarf-wrter -b $STR_JSON $BINARY
```

This data can be used to create or update function entries and the following attributes.

- DW_TAG_subprogram (functions)
    - DW_AT_low_pc
    - DW_AT_name
    - DW_TAG_formal_parameter (arguments)
        - DW_AT_name

## Ghidra functions

Ghidra can export a csv file with info on all functions. To do this go to `Window -> Functions -> select all functions and right click -> Export -> Export to CSV...`. Dwarf-writer can create and update DWARF entries for functions from this data. The following attributes are currently supported.

```
$ dwarf-writer -g $GHIDRA_CSV $BINARY
```

- DW_TAG_subprogram (functions)
    - DW_AT_low_pc
    - DW_AT_high_pc
    - DW_AT_name
    - DW_AT_type
    - DW_TAG_formal_parameter (arguments)
        - DW_AT_name
        - DW_AT_type

There is also experimental support for adding new ELF symbols for functions. If an existing symbol has an address (name) matching a symbol in the input file, the symbol's name (address) is updated. There is currently no support for specifying symbol sections (defaults to ABS).
