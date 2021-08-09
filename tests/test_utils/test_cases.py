from parse_dwarfdump import fn_has_attr, fn_attr_value, var_has_attr, var_attr_value


def test_main_has_name():
    assert fn_has_attr("main", "name", "empty.c.elf")


def test_main_return_type():
    file = "empty.c.elf"
    assert fn_has_attr("main", "type", file)
    assert "int" in fn_attr_value("main", "type", file)


def test_just_loop_name():
    file = "no_return_fn.c.elf"
    assert fn_has_attr("just_loop", "name", file)
    assert fn_has_attr("just_loop", "noreturn", file)


def test_variables():
    file = "types.c.elf"
    assert var_has_attr("xp", "name", file)
    u32_ptr = var_attr_value("xp", "type", file)
    assert "*" in u32_ptr
    assert "int32_t" in u32_ptr
