use std::process::Stdio;

pub struct PrismResult {}

pub struct FeasibleCombination {}

pub fn call_prism(source: &str, prop: &str) -> PrismResult {
    let file_name = "temp.prism";
    let prop_name = "temp.props";
    std::fs::write(file_name, source).expect("Failed to write temporary file");
    std::fs::write(prop_name, prop).expect("Failed to write temporary file");

    let process =
        match std::process::Command::new("/Users/johannes/prism/prism-4.9-mac64-arm/bin/prism")
            .args(&[file_name, prop_name, "--mtbdd"])
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(process) => process,
            Err(err) => panic!("Running process error: {}", err),
        };

    let output = match process.wait_with_output() {
        Ok(output) => output,
        Err(err) => panic!("Retrieving output error: {}", err),
    };

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).unwrap();
        println!("Ran prism successfully");
        println!("{}", stdout);
    } else {
        let stdout = String::from_utf8(output.stdout).unwrap();
        println!("Prism returned an error: {}", stdout)
    }
    todo!();
}
