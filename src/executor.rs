use crate::assertions::{parse_boolean, parse_bytes, parse_integer, parse_comparator, Metric};
use crate::assertions::parse_metric;
use crate::files::empty::empty;
use crate::files::exists::exists;
use crate::files::lines::lines;
use crate::files::size::size;
use crate::parser::Assertion;
use std::path::PathBuf;

pub fn execute(assertion: Assertion) -> Result<(), Box<dyn std::error::Error>> {
    let metric = parse_metric(assertion.metric.as_str())?;
    match metric {
        Metric::FileExists=>{
            let file = PathBuf::from(&assertion.file);
            let comparator = parse_comparator(assertion.comparator.as_str())?;
            let expected = parse_boolean(assertion.expected.as_str())?;
            let actual = exists(&file)?;
            let result = comparator.compare(&actual, &expected);
            let message = format!(
                "Expected {} {} {} {}, got {}",
                assertion.file, metric, comparator, expected, actual
            );
            announce(result, message);
            Ok(())
        },
        Metric::FileSize=>{
            let file = PathBuf::from(&assertion.file);
            let comparator = parse_comparator(assertion.comparator.as_str())?;
            let expected = parse_bytes(assertion.expected.as_str())?;
            let actual = size(&file)?;
            let result = comparator.compare(&actual, &expected);
            let message = format!(
                "Expected {} {} {} {}, got {}",
                assertion.file, metric, comparator, expected, actual
            );
            announce(result, message);
            Ok(())
        },
        Metric::FileEmpty => {
            let file = PathBuf::from(&assertion.file);
            let comparator = parse_comparator(assertion.comparator.as_str())?;
            let expected = parse_boolean(assertion.expected.as_str())?;
            let actual = empty(&file)?;
            let result = comparator.compare(&actual, &expected);
            let message = format!(
                "Expected {} {} {} {}, got {}",
                assertion.file, metric, comparator, expected, actual
            );
            announce(result, message);
            Ok(())
        },
        Metric::FileLines => {
            let file = PathBuf::from(&assertion.file);
            let comparator = parse_comparator(assertion.comparator.as_str())?;
            let expected = parse_integer(assertion.expected.as_str())?;
            let actual = lines(&file)?;
            let result = comparator.compare(&actual, &expected);
            let message = format!(
                "Expected {} {} {} {}, got {}",
                assertion.file, metric, comparator, expected, actual
            );
            announce(result, message);
            Ok(())
        }
    }
}

fn announce(result: bool, message: String) {
    if result {
        println!("PASS. {}", message);
    } else {
        println!("FAIL. {}", message);
    }
}