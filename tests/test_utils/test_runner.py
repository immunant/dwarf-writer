from parse_dwarfdump import *

def test(test_case):
    print("Test " + test_case + " ... ", end='')
    if eval(test_case):
        print("ok")
    else:
        print("FAILED")

tests = open("test_cases", "r")
for t in tests.read().splitlines():
    test(t)
