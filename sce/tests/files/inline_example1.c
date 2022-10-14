#include <stdio.h>

int to_inline(int a, int b) {
    return a + b;
}

int another_inline(int a, int b) {
    int sum = a + b;
    sum++;
    return sum;
}

int main() {
    int x = 1;
    int y = 2;
    int z = to_inline(x, y);
    int w = another_inline(x, y);
    printf("%d\n", z);
    printf("%d\n", w);
    return 0;
}