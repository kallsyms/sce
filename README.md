# Source Slicer

[Slice](https://en.wikipedia.org/wiki/Program_slicing) your source code to make it easier to understand.
It's like [Tantō](https://github.com/Vector35/tanto), but for source code.

[![asciicast](https://asciinema.org/a/QeYyQ9LGwrMwlxftQEJ6uVZWG.svg)](https://asciinema.org/a/QeYyQ9LGwrMwlxftQEJ6uVZWG)

## Slicing Rules

The goal is to lean towards being quick and useful more than implementing correct slicing.
Because we don't do any type of detailed program analysis, we cannot know in general if any given expression
modifies a variable (or a member of a structure). For example:

```c
struct foo *bar = malloc(sizeof(struct foo));
baz(bar);
struct foo x = *bar;
```

Should `baz(bar);` be included in the backward slice of `x`?

This question only gets worse in object oriented languages.
Should a hypothetical `token.refresh()` call be included in a slice of `token`?

In the interest of usefulness, all of these are included.
In fact, any statement with a reference to a variable which is part of a slice is retained in the slice.

This can lead to some unintuitive slices though as demonstrated in [slicer/tests/files/slice2-1.c](./slicer/tests/files/slice2-1.c).
A backward slice on `sum` yields

```c
int i;
int sum = 0;
int w = 7;
for(i = 1; i < N; ++i) {
  sum = sum + i + w;
  product = product * i;
}
write(sum);
```

`product = product * i` is included here as it contains a reference to `i` which is part of the slice since it influences `sum`.


### Big caveats (TODOs):
* There is no awareness of scoping rules right now - everything is based on name alone
* Would a "both" direction be useful? Give context around how this var is initialized and also how it's used later?
* There is currently no sensitivity to where a slice call is made,
i.e. backward slicing on a variable returns the backward slice from the last reference to the variable
and forward slicing returns the forward slice from the first reference to the variable.

A backward slice on `x` in:
```c
int x = 0;
int a = 0;
printf("%d\n", x);
x = a;
printf("%d\n", x);
```

will yield the full contents above regardless "which" `x` is sliced on.


## Architecture

The main slicing code is written in Rust and uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/) for code parsing.
Various IDE plugins (vscode, vim, etc.) can shell out to the Rust binary to easily get slice results.


## License

MIT


## Attributions

* The language detection code and build script was lifted from [difftastic](https://github.com/Wilfred/difftastic)
