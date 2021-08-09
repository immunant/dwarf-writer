#include <stdint.h>

uint32_t x = 0xdeadbeef;
uint32_t* xp = &x;

uint64_t y = 0xcafed00d8badf00d;
uint64_t* yp = &y;
uint64_t** ypp = &yp;

int main() {
    return (int)*xp;
}
