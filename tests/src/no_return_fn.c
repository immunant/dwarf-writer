#include <stdint.h>

uint64_t f() {
    uint64_t x = 0xdeadbeef;
    return x;
}

int main() {
    return f();
}
uint64_t just_loop(uint32_t x) {
    while (1) {}
}
