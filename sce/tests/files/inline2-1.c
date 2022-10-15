// TEST:{"source": "inline_example2.c", "point": [11, 13], "func": "fib", "target": [3, 5]}
#include <stdio.h>

int fib(int n) {
    if (n <= 1) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

int main() {
    if (10 <= 1) {
        // goto line 14
    }
    int x = fib(10 - 1) + fib(10 - 2);
    return 0;
}

