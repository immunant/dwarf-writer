volatile int x = 0;

void just_loop() {
    while (1) {
        x = 0;
    }
}

int main() {
    just_loop();
}
