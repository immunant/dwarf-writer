from parse_dwarfdump import fn_has_attr, fn_attr_value, var_has_attr, var_attr_value

ALL_FILES = ["empty.c.elf", "no_return_fn.c.elf"]


def test_main_has_name():
    for file in ALL_FILES:
        assert fn_has_attr("main", "name", file)


def test_main_return_type():
    for file in ALL_FILES:
        assert "int" in fn_attr_value("main", "type", file)


def test_no_return():
    file = "no_return_fn.c.elf"
    assert fn_has_attr("just_loop", "noreturn", file)
