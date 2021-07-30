import subprocess

default_file = "no_return_fn.c.elf"
tab = "	"

def cmd(cmd, stdin=None):
    return subprocess.run(cmd, input=stdin, stdout=subprocess.PIPE) \
        .stdout \
        .decode("utf-8") \
        .splitlines()

# Get the address of a symbol
def symbol_address(symbol, file=default_file):
    file = "bin/" + file
    nm_out = cmd(["nm", file])
    for line in nm_out:
        line = line.split()
        if line[-1] == symbol:
            return line[0]
    print("Symbol " + symbol + " not found in " + file)

# Find the offset of the first DWARF entry containing a given pattern
def entry_offset(pattern, file=default_file):
    file = "strip_bin/" + file
    full_dump = cmd(["llvm-dwarfdump", file])

    idx = [i for i,x in enumerate(full_dump) if pattern in x][0]
    up_to_pattern = full_dump[0:idx]

    all_offsets = [x for x in up_to_pattern if x.startswith('0x')]
    # Get offset preceding first occurrence of pattern
    last_offset = all_offsets[-1]
    entry_offset = last_offset.split()[0][0:-1]
    return entry_offset

# Get the DWARF entry at the specified offset
def entry_dump(offset, file=default_file):
    file = "strip_bin/" + file
    flag = "--debug-info=" + offset
    return cmd(["llvm-dwarfdump", flag, file])

# Get the llvm-dwarfdump output for the given functions's entry
def find_entry(function, file=default_file):
    addr = symbol_address(function)
    offset = entry_offset("DW_AT_low_pc" + tab + "(0x" + addr + ")")
    return entry_dump(offset, file)

# Get all attributes for a function
def attrs(function, file=default_file):
    entry = find_entry(function, file)
    return [x.lstrip() for x in entry if "DW_AT_" in x]

# Check if a function has a specified attribute
def has_attr(function, attr, file=default_file):
    attr = "DW_AT_" + attr
    for a in attrs(function):
        if a.startswith(attr):
            return True
    return False

# Get the value of a function's attribute
def attr_value(function, attr, file=default_file):
    attr = "DW_AT_" + attr
    for a in attrs(function):
        if a.startswith(attr):
            return ''.join(a.split()[1:])
