int g1;
double g2;

int f1(int *i) {
  return *i + 1;
}

int main() {
  f1(&g1);
  return (int)((void*)&g1 - (void*)&g2);
}
