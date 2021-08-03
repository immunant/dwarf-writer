import os
import subprocess
from typing import Optional, List

DEFAULT_FILE = "no_return_fn.c.elf"
TAB = "	"


def run_cmd(cmd, stdin=None) -> List[str]:
    return subprocess.run(cmd, input=stdin, stdout=subprocess.PIPE, check=False) \
        .stdout \
        .decode("utf-8") \
        .splitlines()


def dwarfdump(*args) -> List[str]:
    assert dwarfdump.cmd, "llvm-dwarfdump not found in path"
    return run_cmd([dwarfdump.cmd] + list(args))


def _find_dwarfdump() -> Optional[str]:
    from shutil import which
    suffixes = ["", "-12", "-11", "-10", "-9"]
    candidates = (f"llvm-dwarfdump{s}" for s in suffixes)
    return next((c for c in candidates if which(c) is not None),
                None)


dwarfdump.cmd = _find_dwarfdump()


def symbol_address(symbol, file=DEFAULT_FILE) -> Optional[str]:
    """
    Get the address of a symbol
    """
    file = os.path.join("bin", file)
    nm_out = run_cmd(["nm", file])
    for line in nm_out:
        line = line.split()
        if line[-1] == symbol:
            return line[0]
    print("Symbol " + symbol + " not found in " + file)
    return None


def entry_offset(pattern, file=DEFAULT_FILE):
    """
    Find the offset of the first DWARF entry containing a given pattern
    """
    file = os.path.join("strip_bin", file)
    full_dump = dwarfdump(file)

    idx = [i for i, x in enumerate(full_dump) if pattern in x][0]
    up_to_pattern = full_dump[0:idx]

    all_offsets = [x for x in up_to_pattern if x.startswith('0x')]
    # Get offset preceding first occurrence of pattern
    # TODO: explain what this means
    last_offset = all_offsets[-1]
    offset = last_offset.split()[0][0:-1]
    return offset


def entry_dump(offset, file=DEFAULT_FILE):
    """
    Get the DWARF entry at the specified offset
    """
    file = os.path.join("strip_bin", file)
    flag = "--debug-info=" + offset
    return dwarfdump(flag, file)


def find_entry(function, file=DEFAULT_FILE):
    """
    Get the llvm-dwarfdump output for the given functions's entry
    """
    addr = symbol_address(function)
    offset = entry_offset(f"DW_AT_low_pc{TAB}(0x{addr})")
    return entry_dump(offset, file)


def attrs(function, file=DEFAULT_FILE):
    """
    Get all attributes for a function
    """
    entry = find_entry(function, file)
    return [x.lstrip() for x in entry if "DW_AT_" in x]


def has_attr(function, attr, file=DEFAULT_FILE):
    """
    Check if a function has a specified attribute
    """
    attr = "DW_AT_" + attr
    for a in attrs(function, file):
        if a.startswith(attr):
            return True
    return False


def attr_value(function, attr, file=DEFAULT_FILE) -> Optional[str]:
    """
    Get the value of a function's attribute
    """
    attr = "DW_AT_" + attr
    for a in attrs(function, file):
        if a.startswith(attr):
            return ''.join(a.split()[1:])
    return None
