# Source Slicer

It's like [Tantō](https://github.com/Vector35/tanto), but for source code.

The goal is to lean towards this being actually useful more than formally correct.

## Architecture

The main slicing code is written in Rust and uses tree-sitter for code parsing.
Various IDE plugins (vscode, vim, etc.) can call into the Rust binary easily to get the slice results.

## License

MIT

## Attributions

* The language detection code and build script was lifted from [difftastic](https://github.com/Wilfred/difftastic)
