//! Compila a parte de runtime da rcheevos (sem cliente HTTP, hash nem
//! rc_client): é o que avalia as condições a cada frame.

use std::path::Path;

fn main() {
    let root = Path::new("rcheevos");
    let runtime_dir = root.join("src/rcheevos");
    if !runtime_dir.is_dir() {
        panic!("submódulo rcheevos ausente: rode `git submodule update --init`");
    }

    let mut build = cc::Build::new();
    build
        .include(root.join("include"))
        .include(root.join("src"))
        .define("RC_STATIC", None)
        .warnings(false);

    for entry in std::fs::read_dir(&runtime_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "c") {
            build.file(&path);
        }
    }
    for file in ["rc_compat.c", "rc_util.c", "rc_version.c"] {
        build.file(root.join("src").join(file));
    }
    // O runtime usa md5 para identificar as definições ativadas.
    build.file(root.join("src/rhash/md5.c"));
    build.compile("rcheevos");

    println!("cargo:rerun-if-changed=rcheevos/src");
    println!("cargo:rerun-if-changed=rcheevos/include");
}
