use insta::assert_snapshot;
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

fn exec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

fn exec_in(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .args(args)
        .current_dir(dir.path())
        .output()
        .expect("failed to run bioassert")
}

fn assertion() -> &'static str {
    "tests/data/empty_file.txt file.exists eq false"
}

fn assertions_file() -> &'static str {
    "tests/data/assertions.txt"
}

fn normalize_log(s: &str) -> String {
    s.lines()
        .map(|line| {
            if let Some(pos) = line.find(' ') {
                let token = &line[..pos];
                if token.ends_with('Z') && token.contains('T') {
                    return format!("[TIMESTAMP]{}", &line[pos..]);
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// --log-file

#[test]
fn explicit_log_file_is_created() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("custom.log");
    exec(&["--log-file", log.to_str().unwrap(), "assert", assertion()]);
    assert!(log.exists(), "expected log file to be created at {}", log.display());
}

#[test]
fn explicit_log_file_snapshot() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("custom.log");
    exec(&["--log-file", log.to_str().unwrap(), "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert_snapshot!("explicit_log_file", normalize_log(&contents));
}

// assert subcommand fallback

#[test]
fn assert_subcommand_writes_assertions_log_in_cwd() {
    let dir = TempDir::new().unwrap();
    exec_in(&dir, &["assert", assertion()]);
    let log = dir.path().join("assertions.log");
    assert!(log.exists(), "expected assertions.log in cwd");
}

#[test]
fn assert_subcommand_log_snapshot() {
    let dir = TempDir::new().unwrap();
    exec_in(&dir, &["assert", assertion()]);
    let contents = std::fs::read_to_string(dir.path().join("assertions.log")).unwrap();
    assert_snapshot!("assert_subcommand_log", normalize_log(&contents));
}

// run subcommand derives log file from assertions file

#[test]
fn run_subcommand_derives_log_file_from_assertions_file() {
    let log = PathBuf::from(format!("{}.log", assertions_file()));
    let _ = std::fs::remove_file(&log);
    exec(&["run", assertions_file()]);
    assert!(log.exists(), "expected {}", log.display());
    let _ = std::fs::remove_file(&log);
}

#[test]
fn run_subcommand_log_snapshot() {
    let log = PathBuf::from(format!("{}.log", "tests/data/failing_assertions.txt"));
    let _ = std::fs::remove_file(&log);
    exec(&["run", "tests/data/failing_assertions.txt"]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert_snapshot!("run_subcommand_log", normalize_log(&contents));
    let _ = std::fs::remove_file(&log);
}

// --verbose writes to stderr as well

#[test]
fn verbose_flag_writes_debug_to_stderr() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--verbose", "assert", assertion()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DEBUG"), "expected DEBUG on stderr: {stderr}");
}

#[test]
fn without_verbose_stderr_has_no_debug_output() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", assertion()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("DEBUG"), "unexpected DEBUG on stderr: {stderr}");
}

// --color-file-log / --color-console-log

fn has_ansi(s: &str) -> bool {
    s.contains("\x1b[")
}

#[test]
fn file_log_has_no_ansi_by_default() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--log-file", log.to_str().unwrap(), "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(!has_ansi(&contents), "expected no ANSI codes in file log by default");
}

#[test]
fn file_log_has_ansi_when_color_file_log_enabled() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--log-file", log.to_str().unwrap(), "--color-file-log=true", "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(has_ansi(&contents), "expected ANSI codes in file log with --color-file-log");
}

#[test]
fn file_log_has_no_ansi_when_color_file_log_disabled_explicitly() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--log-file", log.to_str().unwrap(), "--color-file-log=false", "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(!has_ansi(&contents), "expected no ANSI codes with --color-file-log=false");
}

#[test]
fn verbose_stderr_has_no_ansi_when_color_console_log_disabled() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(
        &dir,
        &["--verbose", "--color-console-log=false", "assert", assertion()],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!has_ansi(&stderr), "expected no ANSI codes on stderr with --color-console-log=false");
}

#[test]
fn verbose_stderr_has_ansi_by_default() {
    // the verbose DEBUG layer keeps level/time/target decorations, which tracing colors
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--verbose", "assert", assertion()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(has_ansi(&stderr), "expected ANSI codes on verbose stderr by default: {stderr:?}");
}

#[test]
fn verbose_stderr_has_ansi_when_color_console_log_enabled_explicitly() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--verbose", "--color-console-log=true", "assert", assertion()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(has_ansi(&stderr), "expected ANSI codes on verbose stderr with --color-console-log=true: {stderr:?}");
}

// --color-console-log colors the PASS / FAIL / ERROR result messages

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";

// passes from a temp dir: file is absent there, so `file.exists eq false` holds
fn passing_assertion() -> &'static str {
    "missing.txt file.exists eq false"
}

// fails from a temp dir: file is absent, so `file.exists eq true` does not hold
fn failing_assertion() -> &'static str {
    "missing.txt file.exists eq true"
}

#[test]
fn pass_message_is_green_by_default() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{GREEN}PASS")), "expected green PASS on stdout: {stdout:?}");
}

#[test]
fn fail_message_is_red_by_default() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", failing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{RED}FAIL")), "expected red FAIL on stdout: {stdout:?}");
}

#[test]
fn error_message_is_red_by_default() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", "this is not valid syntax"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&format!("{RED}ERROR")), "expected red ERROR on stderr: {stderr:?}");
}

#[test]
fn pass_message_is_uncolored_when_console_color_disabled() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color-console-log=false", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS"), "expected PASS text on stdout: {stdout:?}");
    assert!(!has_ansi(&stdout), "expected no ANSI codes on stdout with color disabled: {stdout:?}");
}

#[test]
fn fail_message_is_uncolored_when_console_color_disabled() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color-console-log=false", "assert", failing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FAIL"), "expected FAIL text on stdout: {stdout:?}");
    assert!(!has_ansi(&stdout), "expected no ANSI codes on stdout with color disabled: {stdout:?}");
}

#[test]
fn error_message_is_uncolored_when_console_color_disabled() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color-console-log=false", "assert", "this is not valid syntax"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"), "expected ERROR text on stderr: {stderr:?}");
    assert!(!has_ansi(&stderr), "expected no ANSI codes on stderr with color disabled: {stderr:?}");
}

#[test]
fn pass_message_color_does_not_leak_into_file_log() {
    // console coloring must not appear in the file log (file color is governed separately)
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--log-file", log.to_str().unwrap(), "assert", "tests/data/empty_file.txt file.exists eq true"]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(contents.contains("PASS"), "expected PASS in file log: {contents:?}");
    assert!(!contents.contains(GREEN), "console green must not leak into file log: {contents:?}");
}

// --log-file takes priority over derived path

#[test]
fn explicit_log_file_overrides_derived_path() {
    let dir = TempDir::new().unwrap();
    let explicit = dir.path().join("override.log");
    let derived = PathBuf::from(format!("{}.log", assertions_file()));
    let _ = std::fs::remove_file(&derived);

    exec(&["--log-file", explicit.to_str().unwrap(), "run", assertions_file()]);

    assert!(explicit.exists(), "expected explicit log file");
    assert!(!derived.exists(), "expected derived log file NOT to be created");
}
