use std::process::Command;

const EXPECTED_OUTPUT: &str = "\
Running assertions in tests/assertions.txt
PASS. Expected tests/empty_file.txt file.exists == true, got true
PASS. Expected tests/empty_file.txt file.exists != false, got true
PASS. Expected tests/size_5B.txt file.exists == true, got true
PASS. Expected tests/size_1K.txt file.exists == true, got true
PASS. Expected tests/empty_file.txt file.size == 0B, got 0B
PASS. Expected tests/size_5B.txt file.size == 5B, got 5B
PASS. Expected tests/size_5B.txt file.size == 5B, got 5B
PASS. Expected tests/size_1K.txt file.size == 1.00KB, got 1.00KB
PASS. Expected tests/size_1K.txt file.size == 1.00KB, got 1.00KB
PASS. Expected tests/empty_file.txt file.size != 1B, got 0B
PASS. Expected tests/size_5B.txt file.size != 4B, got 5B
PASS. Expected tests/size_1K.txt file.size != 1023B, got 1.00KB
PASS. Expected tests/empty_file.txt file.size < 1B, got 0B
PASS. Expected tests/size_5B.txt file.size < 6B, got 5B
PASS. Expected tests/size_1K.txt file.size < 2.00KB, got 1.00KB
PASS. Expected tests/size_5B.txt file.size <= 5B, got 5B
PASS. Expected tests/size_5B.txt file.size <= 6B, got 5B
PASS. Expected tests/size_1K.txt file.size <= 1.00KB, got 1.00KB
PASS. Expected tests/size_5B.txt file.size > 4B, got 5B
PASS. Expected tests/size_1K.txt file.size > 500B, got 1.00KB
PASS. Expected tests/size_5B.txt file.size >= 5B, got 5B
PASS. Expected tests/size_5B.txt file.size >= 4B, got 5B
PASS. Expected tests/size_1K.txt file.size >= 1.00KB, got 1.00KB
PASS. Expected tests/empty_file.txt file.empty == true, got true
PASS. Expected tests/empty_file.txt file.empty != false, got true
PASS. Expected tests/size_5B.txt file.empty == false, got false
PASS. Expected tests/size_5B.txt file.empty != true, got false
PASS. Expected tests/size_1K.txt file.empty == false, got false
PASS. Expected tests/empty_file.txt file.lines == 0, got 0
PASS. Expected tests/lines_1.txt file.lines == 1, got 1
PASS. Expected tests/lines_10.txt file.lines == 10, got 10
PASS. Expected tests/lines_100.txt file.lines == 100, got 100
PASS. Expected tests/lines_10.txt file.lines != 9, got 10
PASS. Expected tests/lines_10.txt file.lines != 11, got 10
PASS. Expected tests/lines_10.txt file.lines < 11, got 10
PASS. Expected tests/lines_100.txt file.lines < 101, got 100
PASS. Expected tests/lines_10.txt file.lines <= 10, got 10
PASS. Expected tests/lines_10.txt file.lines <= 11, got 10
PASS. Expected tests/lines_10.txt file.lines > 9, got 10
PASS. Expected tests/lines_100.txt file.lines > 99, got 100
PASS. Expected tests/lines_10.txt file.lines >= 10, got 10
PASS. Expected tests/lines_10.txt file.lines >= 9, got 10";

#[test]
fn run_assertions_file_passes_with_exit_0() {
    let binary = env!("CARGO_BIN_EXE_bioassert");
    let output = Command::new(binary)
        .args(["run", "tests/assertions.txt"])
        .output()
        .expect("failed to run bioassert");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}",
        output.status,
        stdout
    );

    assert_eq!(stdout.trim_end(), EXPECTED_OUTPUT);
}
