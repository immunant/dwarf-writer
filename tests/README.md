# Testing workflow

Typically dwarf-writer is given a single binary plus some disassembly data for it and produces a new binary with updated debug info. For example

```
dwarf-writer -a disasm.json input.elf output.elf
```

where `input.elf` may have symbols and/or existing debug info and `output.elf` has the updated debug info. We can check the result by running `llvm-dwarfdump` on `output.elf` and writing test cases that check the DWARF info for specific entries or attributes and their values.

Let's start by restricting test inputs to binaries with no symbols or existing debug info to test dwarf-writer's most basic functionality. For a given test input `test.c` we'd run

```
clang test.c -o bin/test.elf
strip bin/test.elf -i strip_bin/test.elf
```

Then we generate the disassembly data for the stripped binary and run dwarf-writer on it to produce an updated binary. For example

```
python3 -m anvill --bin_in strip_bin/test.elf --spec_out anvill_json/test.json
cargo run -- -a anvill_json/test.json strip_bin/test.elf out_bin/test.elf
```

At this point we'd run `llvm-dwarfdump` on `out_bin/test.elf` and check the output. Writing test cases to check the output is complicated by the difficulty of manually parsing disassembly data. Even small programs may generate large amounts of data and using stripped binaries means working with addresses instead of names. To get around this we can run `nm` on the binary with symbols `bin/test.elf` to get the address of a function or global variable from its name. This allows us to write test cases by specifying the name of a function/variable in `test.c` and the DWARF attribute it should have like so `fn_has_attr("just_loop", "noreturn")`.
