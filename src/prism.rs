use std::process::Stdio;

pub struct FeasibleCombination {
    pub variables: Vec<String>,
    value: String,
}

pub fn call_prism(source: &str, prop: &str) -> Vec<FeasibleCombination> {
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
        let isolated = isolate_results(&stdout)
            .unwrap_or_else(|| panic!("Failed to parse prism output: \n{}", stdout));
        parse_results(isolated)
    } else {
        let stdout = String::from_utf8(output.stdout).unwrap();
        panic!("Prism returned an error: {}", stdout)
    }
}

fn isolate_results(stdout: &str) -> Option<&str> {
    let start_search = "Results (non-zero only) for filter \"init\":\n";
    let end_search = "Range of values over initial states: ";
    let start_index = stdout.find(start_search)? + start_search.len();
    let end_index = stdout[start_index..].find(end_search)? + start_index;
    Some(stdout[start_index..end_index].trim())
}

fn parse_results(results: &str) -> Vec<FeasibleCombination> {
    if results == "(all zero)" {
        Vec::new()
    } else {
        results
            .lines()
            .map(|l| to_feasible_combination(l))
            .collect()
    }
}

fn to_feasible_combination(entry: &str) -> FeasibleCombination {
    if let Some((_state_id, info)) = entry.split_once(":") {
        if let Some((valuation, value)) = info.split_once("=") {
            let trimmed = info.trim();
            let inner_valuation = &trimmed[1..trimmed.len() - 1];
            let variables = inner_valuation
                .split(",")
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            FeasibleCombination {
                variables,
                value: value.to_string(),
            }
        } else {
            panic!("Feasible combination should include `=`");
        }
    } else {
        panic!("Feasible state should contain `:`");
    }
}
