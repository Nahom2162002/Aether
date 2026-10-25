// Runs every examples/*.ae and compares stdout with the matching .out file.
// After adding a feature: write examples/NN_name.ae, check its output by hand,
// then save it with `cargo run -- examples/NN_name.ae > examples/NN_name.out`.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn examples_match_expected_output() {
	let mut checked = 0;
	for entry in fs::read_dir("examples").unwrap() {
		let path = entry.unwrap().path();
		if path.extension().and_then(|e| e.to_str()) != Some("ae") {
			continue;
		}
		let expected_path = path.with_extension("out");
		assert!(
			Path::new(&expected_path).exists(),
			"missing expected output {}",
			expected_path.display()
		);

		let output = Command::new(env!("CARGO_BIN_EXE_aether"))
			.arg(&path)
			.output()
			.unwrap();
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(output.status.success(), "{} failed:\n{}", path.display(), stderr);

		let normalize = |s: &str| s.replace("\r\n", "\n");
		let actual = normalize(&String::from_utf8_lossy(&output.stdout));
		let expected = normalize(&fs::read_to_string(&expected_path).unwrap());
		assert_eq!(actual, expected, "output mismatch for {}", path.display());
		checked += 1;
	}
	assert!(checked > 0, "no examples found");
}
