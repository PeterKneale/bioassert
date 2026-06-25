use crate::core::{Comparator, strip_quotes};

/// One suggested assertion line, the structured form of a `suggest` output line.
///
/// The comparator is stored as its keyword form (`eq`, `gt`, `gte`, `lte`) rather than as a
/// [`Comparator`], because [`Comparator`]'s `Display` renders symbols (`==`, `>=`) while an
/// assertions file needs keywords. The expected value is pre-rendered: each provider computes
/// a `Value` from the existing property functions and renders it once via `Value::Display`, or
/// renders a derived band integer directly, so this struct never has to know how a value is
/// formatted.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    /// The resource (file path) as passed to `suggest`.
    pub resource: String,
    /// The metric, e.g. `file.size`, `tsv.lines.count`.
    pub metric: String,
    /// The comparator keyword: `eq`, `gt`, `gte`, or `lte`.
    pub comparator: &'static str,
    /// The rendered expected value, e.g. `0B`, `12`, `true`.
    pub expected: String,
    /// Optional inline explanation appended as `  # comment`.
    pub comment: Option<String>,
}

impl Suggestion {
    /// Builds a suggestion. The comparator must be a keyword the engine can parse back; a
    /// typo is caught in debug builds by the `debug_assert!`.
    pub fn new(
        resource: impl Into<String>,
        metric: impl Into<String>,
        comparator: &'static str,
        expected: impl Into<String>,
        comment: Option<&str>,
    ) -> Self {
        debug_assert!(
            comparator.parse::<Comparator>().is_ok(),
            "comparator keyword does not parse: {comparator}"
        );
        Self {
            resource: resource.into(),
            metric: metric.into(),
            comparator,
            expected: expected.into(),
            comment: comment.map(str::to_string),
        }
    }

    /// Renders the suggestion as one assertion line: `resource metric comparator expected`,
    /// with `  # comment` appended when present. The resource is single-quoted when it
    /// contains whitespace and is not already quoted, mirroring the locator-quoting rule the
    /// parser enforces, so the line round-trips through the grammar.
    pub fn render(&self) -> String {
        let mut line = format!(
            "{} {} {} {}",
            quote_resource(&self.resource),
            self.metric,
            self.comparator,
            self.expected
        );
        if let Some(comment) = &self.comment {
            line.push_str("  # ");
            line.push_str(comment);
        }
        line
    }
}

/// Single-quotes `resource` when it contains whitespace and is not already quoted.
fn quote_resource(resource: &str) -> String {
    if resource.chars().any(char::is_whitespace) && strip_quotes(resource) == resource {
        format!("'{resource}'")
    } else {
        resource.to_string()
    }
}

/// The +/- 50% band around `n` as two integer bounds, `(floor(0.5n), ceil(1.5n))`. Used to
/// suggest a lower (`gte`) and upper (`lte`) bound for a quantity that varies run to run.
/// Integer arithmetic: `band(4) = (2, 6)`, `band(3) = (1, 5)`, `band(0) = (0, 0)`.
pub fn band(n: u64) -> (u64, u64) {
    // floor(0.5n) = n / 2; ceil(1.5n) = n + ceil(0.5n) = n + n.div_ceil(2).
    (n / 2, n + n.div_ceil(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_without_a_comment() {
        let s = Suggestion::new("data.tsv", "tsv.lines.count", "lte", "6", None);
        assert_eq!(s.render(), "data.tsv tsv.lines.count lte 6");
    }

    #[test]
    fn renders_with_a_comment() {
        let s = Suggestion::new(
            "data.tsv",
            "file.exists",
            "eq",
            "true",
            Some("file is present"),
        );
        assert_eq!(
            s.render(),
            "data.tsv file.exists eq true  # file is present"
        );
    }

    #[test]
    fn single_quotes_a_resource_with_a_space() {
        let s = Suggestion::new("my output.tsv", "file.exists", "eq", "true", None);
        assert_eq!(s.render(), "'my output.tsv' file.exists eq true");
    }

    #[test]
    fn does_not_requote_an_already_quoted_resource() {
        let s = Suggestion::new("'my output.tsv'", "file.exists", "eq", "true", None);
        assert_eq!(s.render(), "'my output.tsv' file.exists eq true");
    }

    #[test]
    fn keeps_the_comparator_keyword_with_no_symbol_leakage() {
        for keyword in ["eq", "gt", "gte", "lte"] {
            let s = Suggestion::new("data.tsv", "file.size", keyword, "0B", None);
            assert_eq!(s.comparator, keyword);
            assert!(!s.render().contains(['<', '>', '=']));
        }
    }

    #[test]
    fn band_covers_representative_values() {
        assert_eq!(band(0), (0, 0));
        assert_eq!(band(1), (0, 2));
        assert_eq!(band(2), (1, 3));
        assert_eq!(band(3), (1, 5));
        assert_eq!(band(4), (2, 6));
        assert_eq!(band(100), (50, 150));
    }
}
