#include <stdbool.h>

typedef struct thing {
    int x;
    int bar;
} thing;

int main() {
    int x = 0;
    int y = 0;
    int z = x;
    if (true) {
        struct thing foo = (struct thing){
            .x = x,
        };
        foo.bar = 0;
    }

    return z;
}
