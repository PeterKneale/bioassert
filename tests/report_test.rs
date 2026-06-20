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

// Writes a self-contained assertions file into `dir` and returns its path. The
// data path is relative to the repo root, where the test binary runs, so the
// derived `.log` lands inside `dir` rather than the shared `tests/data` tree.
fn write_assertions(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("assertions.txt");
    std::fs::write(&path, "tests/data/empty_file.txt file.exists eq true\n").unwrap();
    path
}

// The assertion report is the only file bioassert writes. Its path is resolved from
// --report-file, the derived <file>.log for `run`, or assertions.log by default.

#[test]
fn explicit_report_file_is_created() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("custom.log");
    exec(&["--report-file", log.to_str().unwrap(), "assert", assertion()]);
    assert!(log.exists(), "expected report file to be created at {}", log.display());
}

#[test]
fn explicit_report_file_snapshot() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("custom.log");
    exec(&["--report-file", log.to_str().unwrap(), "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert_snapshot!("explicit_report_file", contents);
}

// assert subcommand falls back to assertions.log in the cwd

#[test]
fn assert_subcommand_writes_assertions_log_in_cwd() {
    let dir = TempDir::new().unwrap();
    exec_in(&dir, &["assert", assertion()]);
    let log = dir.path().join("assertions.log");
    assert!(log.exists(), "expected assertions.log in cwd");
}

#[test]
fn assert_subcommand_report_snapshot() {
    let dir = TempDir::new().unwrap();
    exec_in(&dir, &["assert", assertion()]);
    let contents = std::fs::read_to_string(dir.path().join("assertions.log")).unwrap();
    assert_snapshot!("assert_subcommand_report", contents);
}

// run subcommand derives the report path from the assertions file

#[test]
fn run_subcommand_derives_report_file_from_assertions_file() {
    let dir = TempDir::new().unwrap();
    let file = write_assertions(&dir);
    exec(&["run", file.to_str().unwrap()]);
    let log = dir.path().join("assertions.txt.log");
    assert!(log.exists(), "expected {}", log.display());
}

#[test]
fn run_subcommand_report_snapshot() {
    let log = PathBuf::from(format!("{}.log", "tests/data/failing_assertions.txt"));
    let _ = std::fs::remove_file(&log);
    exec(&["run", "tests/data/failing_assertions.txt"]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert_snapshot!("run_subcommand_report", contents);
    let _ = std::fs::remove_file(&log);
}

fn has_ansi(s: &str) -> bool {
    s.contains("\x1b[")
}

// Runs the binary with the given args and environment overrides. Used by the
// NO_COLOR precedence test; the plain `exec` helpers inherit the parent env.
fn exec_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bioassert"));
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("failed to run bioassert")
}

// The report file is always plain text, independent of the --color console setting,
// so it stays parseable by downstream tools.

#[test]
fn report_file_is_uncolored_by_default() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--report-file", log.to_str().unwrap(), "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(!has_ansi(&contents), "expected no ANSI codes in report file by default");
}

#[test]
fn report_file_is_uncolored_even_when_console_color_is_always() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--report-file", log.to_str().unwrap(), "--color=always", "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(!has_ansi(&contents), "console color must never reach the report file: {contents:?}");
}

// --color <auto|always|never> colors the PASS / FAIL / ERROR result keywords on the
// console. `auto` (the default) is off when output is piped (not a terminal), which is
// exactly the case for these subprocess tests.

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
fn color_is_off_by_default_when_not_a_terminal() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS"), "expected PASS text on stdout: {stdout:?}");
    assert!(!has_ansi(&stdout), "expected no ANSI on piped stdout under auto: {stdout:?}");
}

#[test]
fn pass_message_is_green_when_color_always() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color=always", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{GREEN}PASS")), "expected green PASS on stdout: {stdout:?}");
}

#[test]
fn fail_message_is_red_when_color_always() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color=always", "assert", failing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{RED}FAIL")), "expected red FAIL on stdout: {stdout:?}");
}

#[test]
fn error_message_is_red_when_color_always() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color=always", "assert", "this is not valid syntax"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&format!("{RED}ERROR")), "expected red ERROR on stderr: {stderr:?}");
}

#[test]
fn pass_message_is_uncolored_when_color_never() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color=never", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS"), "expected PASS text on stdout: {stdout:?}");
    assert!(!has_ansi(&stdout), "expected no ANSI codes on stdout with --color=never: {stdout:?}");
}

#[test]
fn fail_message_is_uncolored_when_color_never() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color=never", "assert", failing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FAIL"), "expected FAIL text on stdout: {stdout:?}");
    assert!(!has_ansi(&stdout), "expected no ANSI codes on stdout with --color=never: {stdout:?}");
}

#[test]
fn error_message_is_uncolored_when_color_never() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--color=never", "assert", "this is not valid syntax"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"), "expected ERROR text on stderr: {stderr:?}");
    assert!(!has_ansi(&stderr), "expected no ANSI codes on stderr with --color=never: {stderr:?}");
}

// --icons <auto|always|never> mirrors --color: it prefixes PASS / FAIL / ERROR
// result lines with a status icon (🟢 / 🔴 / 🔥). Like color, `auto` (the default)
// is off when output is not a terminal, which is the case for these subprocess tests.

#[test]
fn pass_line_has_icon_when_icons_always() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--icons=always", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🟢  PASS"), "expected pass icon on stdout: {stdout:?}");
}

#[test]
fn fail_line_has_icon_when_icons_always() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--icons=always", "assert", failing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🔴  FAIL"), "expected fail icon on stdout: {stdout:?}");
}

#[test]
fn error_line_has_flame_when_icons_always() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--icons=always", "assert", "this is not valid syntax"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("🔥  ERROR"), "expected flame icon on stderr: {stderr:?}");
}

#[test]
fn icons_are_off_by_default_when_not_a_terminal() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS"), "expected PASS text on stdout: {stdout:?}");
    assert!(!stdout.contains("🟢"), "expected no icon under auto when piped: {stdout:?}");
}

#[test]
fn icons_never_disables_even_on_a_terminal() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--icons=never", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("🟢"), "expected no icon with --icons=never: {stdout:?}");
}

// icons and color are resolved independently: each can be on while the other is off.

#[test]
fn icons_without_color() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--icons=always", "--color=never", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🟢  PASS"), "expected icon without color: {stdout:?}");
    assert!(!has_ansi(&stdout), "expected no ANSI with --color=never: {stdout:?}");
}

#[test]
fn color_without_icons() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--icons=never", "--color=always", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{GREEN}PASS")), "expected green PASS: {stdout:?}");
    assert!(!stdout.contains("🟢"), "expected no icon with --icons=never: {stdout:?}");
}

#[test]
fn report_file_has_no_icons_even_when_icons_always() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("out.log");
    exec(&["--report-file", log.to_str().unwrap(), "--icons=always", "assert", assertion()]);
    let contents = std::fs::read_to_string(&log).unwrap();
    assert!(!contents.contains("🟢") && !contents.contains("🔴") && !contents.contains("🔥"),
        "expected no icons in report file: {contents:?}");
}

// `--colour` is accepted as a spelling alias for `--color`.

#[test]
fn colour_alias_is_accepted() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["--colour=always", "assert", passing_assertion()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{GREEN}PASS")), "expected --colour to color output: {stdout:?}");
}

// NO_COLOR precedence: an explicit --color=always overrides the environment

#[test]
fn color_always_overrides_no_color_env() {
    let output = exec_with_env(&["--color=always", "assert", passing_assertion()], &[("NO_COLOR", "1")]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(has_ansi(&stdout), "explicit --color=always must override NO_COLOR: {stdout:?}");
}

// --report-file takes priority over the derived path

#[test]
fn explicit_report_file_overrides_derived_path() {
    let dir = TempDir::new().unwrap();
    let file = write_assertions(&dir);
    let explicit = dir.path().join("override.log");

    exec(&["--report-file", explicit.to_str().unwrap(), "run", file.to_str().unwrap()]);

    let derived = dir.path().join("assertions.txt.log");
    assert!(explicit.exists(), "expected explicit report file");
    assert!(!derived.exists(), "expected derived report file NOT to be created");
}

// The global options are accepted after the subcommand and its arguments, not only
// before it, so `assert <str> --colour=always` works the same as `--colour=always assert <str>`.

#[test]
fn global_flags_are_accepted_after_the_subcommand() {
    let dir = TempDir::new().unwrap();
    let output = exec_in(&dir, &["assert", passing_assertion(), "--color=always", "--icons=always"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🟢"), "expected icon from trailing --icons: {stdout:?}");
    assert!(stdout.contains(&format!("{GREEN}PASS")), "expected color from trailing --color: {stdout:?}");
}

#[test]
fn report_file_flag_is_accepted_after_the_subcommand() {
    let dir = TempDir::new().unwrap();
    let file = write_assertions(&dir);
    let explicit = dir.path().join("trailing.log");
    exec(&["run", file.to_str().unwrap(), "--report-file", explicit.to_str().unwrap()]);
    assert!(explicit.exists(), "expected trailing --report-file to be honored");
}
