import os
import subprocess

default_file = "no_return_fn.c.elf"
tab = "	"


def cmd(cmd, stdin=None):
    return subprocess.run(cmd, input=stdin, stdout=subprocess.PIPE) \
        .stdout \
        .decode("utf-8") \
        .splitlines()


def symbol_address(symbol, file=default_file):
    """
    Get the address of a symbol
    """
    file = os.path.join("bin", file)
    nm_out = cmd(["nm", file])
    for line in nm_out:
        line = line.split()
        if line[-1] == symbol:
            return line[0]
    print("Symbol " + symbol + " not found in " + file)


def entry_offset(pattern, file=default_file):
    """
    Find the offset of the first DWARF entry containing a given pattern
    """
    file = os.path.join("strip_bin", file)
    full_dump = cmd(["llvm-dwarfdump-12", file])

    idx = [i for i, x in enumerate(full_dump) if pattern in x][0]
    up_to_pattern = full_dump[0:idx]

    all_offsets = [x for x in up_to_pattern if x.startswith('0x')]
    # Get offset preceding first occurrence of pattern
    last_offset = all_offsets[-1]
    entry_offset = last_offset.split()[0][0:-1]
    return entry_offset


def entry_dump(offset, file=default_file):
    """
    Get the DWARF entry at the specified offset
    """
    file = os.path.join("strip_bin", file)
    flag = "--debug-info=" + offset
    return cmd(["llvm-dwarfdump-12", flag, file])


def find_entry(function, file=default_file):
    """
    Get the llvm-dwarfdump output for the given functions's entry
    """
    addr = symbol_address(function)
    offset = entry_offset(f"DW_AT_low_pc{tab}(0x{addr})")
    return entry_dump(offset, file)


def attrs(function, file=default_file):
    """
    Get all attributes for a function
    """
    entry = find_entry(function, file)
    return [x.lstrip() for x in entry if "DW_AT_" in x]


def has_attr(function, attr, file=default_file):
    """
    Check if a function has a specified attribute
    """
    attr = "DW_AT_" + attr
    for a in attrs(function):
        if a.startswith(attr):
            return True
    return False


def attr_value(function, attr, file=default_file):
    """
    Get the value of a function's attribute
    """
    attr = "DW_AT_" + attr
    for a in attrs(function):
        if a.startswith(attr):
            return ''.join(a.split()[1:])
