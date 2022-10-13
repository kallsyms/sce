// adapted from https://github.com/Wilfred/difftastic/blob/master/build.rs
use rayon::prelude::*;
use std::path::PathBuf;

struct TreeSitterParser {
    name: &'static str,
    src_dir: &'static str,
    extra_files: Vec<&'static str>,
}

impl TreeSitterParser {
    fn build(&self) {
        let dir = PathBuf::from(&self.src_dir);

        let mut c_files = vec!["parser.c"];
        let mut cpp_files = vec![];

        for file in &self.extra_files {
            if file.ends_with(".c") {
                c_files.push(file);
            } else {
                cpp_files.push(file);
            }
        }

        let mut build = cc::Build::new();
        build.include(&dir).warnings(false); // ignore unused parameter warnings
        for file in c_files {
            build.file(dir.join(file));
        }
        build.compile(self.name);

        if !cpp_files.is_empty() {
            let mut cpp_build = cc::Build::new();
            cpp_build
                .include(&dir)
                .cpp(true)
                .flag_if_supported("-Wno-implicit-fallthrough")
                .flag_if_supported("-Wno-unused-parameter")
                .flag_if_supported("-Wno-ignored-qualifiers")
                // Workaround for: https://github.com/ganezdragon/tree-sitter-perl/issues/16
                // should be removed after fixed.
                .flag_if_supported("-Wno-return-type");

            if cfg!(windows) {
                cpp_build.flag("/std:c++14");
            } else {
                cpp_build.flag("--std=c++14");
            }

            for file in cpp_files {
                cpp_build.file(dir.join(file));
            }
            cpp_build.compile(&format!("{}-cpp", self.name));
        }
    }
}

fn main() {
    let parsers = vec![
        TreeSitterParser {
            name: "tree-sitter-c",
            src_dir: "vendor/tree-sitter-c/src",
            extra_files: vec![],
        },
        TreeSitterParser {
            name: "tree-sitter-cpp",
            src_dir: "vendor/tree-sitter-cpp/src",
            extra_files: vec!["scanner.cc"],
        },
        TreeSitterParser {
            name: "tree-sitter-c-sharp",
            src_dir: "vendor/tree-sitter-c-sharp/src",
            extra_files: vec!["scanner.c"],
        },
        TreeSitterParser {
            name: "tree-sitter-go",
            src_dir: "vendor/tree-sitter-go/src",
            extra_files: vec![],
        },
        TreeSitterParser {
            name: "tree-sitter-java",
            src_dir: "vendor/tree-sitter-java/src",
            extra_files: vec![],
        },
        TreeSitterParser {
            name: "tree-sitter-javascript",
            src_dir: "vendor/tree-sitter-javascript/src",
            extra_files: vec!["scanner.c"],
        },
        TreeSitterParser {
            name: "tree-sitter-python",
            src_dir: "vendor/tree-sitter-python/src",
            extra_files: vec!["scanner.cc"],
        },
        TreeSitterParser {
            name: "tree-sitter-ruby",
            src_dir: "vendor/tree-sitter-ruby/src",
            extra_files: vec!["scanner.cc"],
        },
        TreeSitterParser {
            name: "tree-sitter-rust",
            src_dir: "vendor/tree-sitter-rust/src",
            extra_files: vec!["scanner.c"],
        },
        TreeSitterParser {
            name: "tree-sitter-tsx",
            src_dir: "vendor/tree-sitter-typescript/tsx/src",
            extra_files: vec!["scanner.c"],
        },
        TreeSitterParser {
            name: "tree-sitter-typescript",
            src_dir: "vendor/tree-sitter-typescript/typescript/src",
            extra_files: vec!["scanner.c"],
        },
    ];

    // Only rerun if relevant files in the vendor/ directory change.
    for parser in &parsers {
        println!("cargo:rerun-if-changed={}", parser.src_dir);
    }

    parsers.par_iter().for_each(|p| p.build());

    tonic_build::compile_protos("../sce.proto").unwrap();
}
