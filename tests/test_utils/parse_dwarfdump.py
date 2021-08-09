import os
import subprocess
from typing import Optional, List

TAB = "	"


def run_cmd(cmd, stdin=None) -> List[str]:
    return (
        subprocess.run(cmd, input=stdin, stdout=subprocess.PIPE, check=False)
        .stdout.decode("utf-8")
        .splitlines()
    )


def dwarfdump(*args) -> List[str]:
    print(args)
    assert dwarfdump.cmd, "llvm-dwarfdump not found in path"
    return run_cmd([dwarfdump.cmd] + list(args))


def _find_dwarfdump() -> Optional[str]:
    from shutil import which

    suffixes = ["", "-12", "-11", "-10", "-9"]
    candidates = (f"llvm-dwarfdump{s}" for s in suffixes)
    return next((c for c in candidates if which(c) is not None), None)


dwarfdump.cmd = _find_dwarfdump()


def symbol_address(symbol, full_path) -> Optional[str]:
    """
    Get the address of a symbol
    """
    nm_out = run_cmd(["nm", full_path])
    for line in nm_out:
        line = line.split()
        if line[-1] == symbol:
            return line[0]
    print("Symbol " + symbol + " not found in " + full_path)
    return None


def entry_dump(symbol, sym_type, file):
    """
    Get the llvm-dwarfdump output for the given functions or variable's entry
    """
    updated_file = os.path.join("out_bin", "strip." + file)
    sym_file = os.path.join("bin", file)
    full_dump = dwarfdump(updated_file)

    blank_lines = [i for i, line in enumerate(full_dump) if line == ""]
    addr = symbol_address(symbol, sym_file)
    if sym_type == "function":
        pattern = f"DW_AT_low_pc{TAB}(0x{addr})"
    elif sym_type == "variable":
        addr = addr.lstrip("0")
        pattern = f"DW_AT_location{TAB}(DW_OP_addr 0x{addr})"
    else:
        return None
    pattern = (" " * 16) + pattern
    idx = full_dump.index(pattern)
    for n in blank_lines:
        if idx < n:
            end = n
            break
    start = blank_lines[blank_lines.index(end) - 1]
    return full_dump[start + 1 : end]


def _attrs(symbol, sym_type, file):
    """
    Get all attributes for a function or variable
    """
    entry = entry_dump(symbol, sym_type, file)
    return [x.lstrip() for x in entry if "DW_AT_" in x]


def _has_attr(symbol, sym_type, attr, file):
    """
    Check if a function or variable has a specified attribute
    """
    attr = "DW_AT_" + attr
    for a in _attrs(symbol, sym_type, file):
        if a.startswith(attr):
            return True
    return False


def _attr_value(symbol, sym_type, attr, file) -> Optional[str]:
    """
    Get the value of a function or variable's attribute
    """
    attr = "DW_AT_" + attr
    for a in _attrs(symbol, sym_type, file):
        if a.startswith(attr):
            attr_value = a.split()[1:]
            return " ".join(attr_value)
    return None


def fn_attrs(symbol, file):
    return _attrs(symbol, "function", file)


def fn_has_attr(symbol, attr, file):
    return _has_attr(symbol, "function", attr, file)


def fn_attr_value(symbol, attr, file):
    return _attr_value(symbol, "function", attr, file)


def var_attrs(symbol, file):
    return _attrs(symbol, "variable", file)


def var_has_attr(symbol, attr, file):
    return _has_attr(symbol, "variable", attr, file)


def var_attr_value(symbol, attr, file):
    return _attr_value(symbol, "variable", attr, file)
