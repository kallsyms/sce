// TEST:{"source": "example2.c", "point": [13, 11], "var": "sum", "direction": "Backward"}
// Sample from the wikipedia page on program slicing: https://en.wikipedia.org/wiki/Program_slicing
#define N 100

int main() {
    int i;
    int sum = 0;
    int w = 7;
    for(i = 1; i < N; ++i) {
      sum = sum + i + w;
      product = product * i;
    }
    write(sum);
}
