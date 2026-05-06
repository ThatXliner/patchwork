use std::io::Write;
use std::process::Command;

fn patchwork() -> Command {
    Command::new(env!("CARGO_BIN_EXE_patchwork"))
}

fn run_pipe(args: &[&str], input: &str) -> Result<String, String> {
    let mut child = patchwork()
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn run_with_args(args: &[&str]) -> Result<String, String> {
    let output = patchwork().args(args).output().unwrap();
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// ——— find ———

#[test]
fn test_find_stdin_java() {
    let out = run_pipe(&["find", "-l", "java", "-p", "return 1;"], "class A { void f() { return 1; } }").unwrap();
    assert!(out.contains("return 1;"));
}

#[test]
fn test_find_stdin_python() {
    let out = run_pipe(&["find", "-l", "python", "-p", "x = 42"], "x = 42\n").unwrap();
    assert!(out.contains("x = 42"));
}

#[test]
fn test_find_no_matches() {
    let out = run_pipe(&["find", "-l", "java", "-p", "return 1;"], "class A { void f() { } }").unwrap();
    assert!(out.is_empty());
}

#[test]
fn test_find_requires_language_on_stdin() {
    let result = run_pipe(&["find", "-p", "return 1;"], "class A {}");
    assert!(result.is_err());
}

// ——— replace ———

#[test]
fn test_replace_stdin() {
    let out = run_pipe(&["replace", "-l", "java", "-p", "return 1;", "-r", "return 2;"], "class A { void f() { return 1; } }").unwrap();
    assert!(out.contains("return 2;"));
    assert!(!out.contains("return 1;"));
}

#[test]
fn test_replace_in_place() -> std::io::Result<()> {
    let dir = std::env::temp_dir().join(format!("patchwork_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let file_path = dir.join("Test.java");
    std::fs::write(&file_path, "class A { void f() { return 1; } }")?;

    let output = patchwork()
        .args(&["replace", "-i", "-l", "java", "-p", "return 1;", "-r", "return 99;"])
        .arg(file_path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(output.status.success(), "replace -i failed: {}", String::from_utf8_lossy(&output.stderr));

    let result = std::fs::read_to_string(&file_path)?;
    assert!(result.contains("return 99;"), "result: {}", result);
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

// ——— insert-after ———

#[test]
fn test_insert_after_stdin() {
    let out = run_pipe(
        &["insert-after", "-l", "java", "-p", "return 1;", "--code", "\nreturn 2;"],
        "class A { void f() { return 1; } }",
    ).unwrap();
    assert!(out.contains("return 2;"));
}

// ——— delete ———

#[test]
fn test_delete_stdin() {
    let out = run_pipe(&["delete", "-l", "java", "-p", "return 1;"], "class A { void f() { return 1; } }").unwrap();
    assert!(!out.contains("return 1;"));
}

// ——— query mode ———

#[test]
fn test_find_query_java() {
    let out = run_pipe(
        &["find", "-l", "java", "-q", "(method_declaration name: (identifier) @name)"],
        "class A { void f() {} }",
    ).unwrap();
    assert!(out.contains("f"));
}

// ——— misc ———

#[test]
fn test_help_succeeds() {
    let out = run_with_args(&["--help"]).unwrap();
    assert!(out.contains("patchwork"));
}

// ——— error cases ———

#[test]
fn test_no_pattern_or_query_fails() {
    let result = run_pipe(&["find", "-l", "java"], "class A {}");
    assert!(result.is_err());
}

#[test]
fn test_in_place_requires_files() {
    let result = run_pipe(&["replace", "-i", "-l", "java", "-p", "x", "-r", "y"], "class A {}");
    assert!(result.is_err());
}
