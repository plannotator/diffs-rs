use super::syntax_for_path;

fn grammar(path: &str) -> Option<String> {
    let syntaxes = two_face::syntax::extra_newlines();
    syntax_for_path(path, &syntaxes).map(|syntax| syntax.name.clone())
}

#[test]
fn whole_file_names_select_grammars() {
    for (name, expected) in [
        ("Makefile", "Makefile"),
        ("GNUmakefile", "Makefile"),
        ("Dockerfile", "Dockerfile"),
        ("Cargo.lock", "TOML"),
        ("Pipfile", "TOML"),
        ("Gemfile", "Ruby"),
        ("Rakefile", "Ruby"),
        ("Vagrantfile", "Ruby"),
        (".bashrc", "Bourne Again Shell (bash)"),
        (".zshrc", "Bourne Again Shell (bash)"),
        ("PKGBUILD", "Bourne Again Shell (bash)"),
        ("CMakeLists.txt", "CMake"),
        ("Jenkinsfile", "Groovy"),
        (".gitignore", "Git Ignore"),
        ("go.mod", "Gomod"),
    ] {
        for path in [name.to_owned(), format!("nested dir/{name}")] {
            assert_eq!(grammar(&path).as_deref(), Some(expected), "{path}");
        }
    }
}

#[test]
fn file_names_take_precedence_over_extensions() {
    // `.txt` selects plain text and `.lock` has no grammar, unlike `CMakeLists.txt` and `Cargo.lock`.
    assert_eq!(grammar("notes.txt").as_deref(), Some("Plain Text"));
    assert_eq!(grammar("other.lock"), None);
}

#[test]
fn extensions_still_select_grammars_without_case() {
    assert_eq!(grammar("src/main.rs").as_deref(), Some("Rust"));
    assert_eq!(grammar("SCRIPT.PY").as_deref(), Some("Python"));
    assert_eq!(grammar("Makefile.am").as_deref(), Some("Makefile"));
    assert_eq!(grammar("archive.tar.gz"), None);
    assert_eq!(grammar("README"), None);
    assert_eq!(grammar(""), None);
}

#[test]
fn whole_file_names_are_case_sensitive() {
    // Bazel `BUILD` files must not take the grammar registered for `.build` extensions.
    assert_eq!(grammar("BUILD"), None);
    assert_eq!(grammar("project.build").as_deref(), Some("NAnt Build File"));
}
