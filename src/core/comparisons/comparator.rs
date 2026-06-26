use super::errors::ComparatorError;
use super::errors::ComparatorError::UnknownComparator;
use crate::core::errors::BioAssertError;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// The comparison operator, without negation. This is the closed set the grammar's
/// `comparator_op` rule accepts.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Operator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Starts,
    Ends,
    Contains,
    Matches,
}

/// A comparison operator plus an optional `not` modifier. Negation is applied at the
/// comparison itself (not at the assertion's final result), so it composes correctly with
/// the whole-column aggregates: a negated matcher applied per cell yields "no cell
/// matches" rather than the De Morgan dual "some cell does not match".
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Comparator {
    pub op: Operator,
    pub negate: bool,
}

impl Comparator {
    /// A comparator with no negation. The grammar produces the `negate` flag, so this is
    /// mainly a convenience for tests and direct construction.
    pub fn new(op: Operator) -> Self {
        Self { op, negate: false }
    }

    pub fn compare<T>(self, actual: T, expected: T) -> bool
    where
        T: PartialOrd + PartialEq,
    {
        match self.op {
            Operator::Eq => (actual == expected) ^ self.negate,
            Operator::Ne => (actual != expected) ^ self.negate,
            Operator::Lt => (actual < expected) ^ self.negate,
            Operator::Le => (actual <= expected) ^ self.negate,
            Operator::Gt => (actual > expected) ^ self.negate,
            Operator::Ge => (actual >= expected) ^ self.negate,
            // The string operators are not valid for `compare` (the numeric/boolean path).
            // Return false regardless of `negate`, so a mis-paired `not contains` on a
            // numeric metric stays an always-FAIL (a visible mistake) rather than flipping
            // to an always-PASS that would silently hide the error in a validation gate.
            _ => false,
        }
    }

    pub fn compare_string(self, actual: &str, expected: &str) -> Result<bool, BioAssertError> {
        Ok(self.string_matcher(expected)?.is_match(actual))
    }

    /// Builds a reusable [`StringMatcher`] for `expected`, compiling any regex exactly
    /// once. Use this instead of [`Self::compare_string`] when the same comparison is
    /// applied to many values (e.g. every cell of a delimited column) so a `matches`
    /// pattern is not recompiled per value. Errors if the comparator is not valid for
    /// string comparison (the numeric comparators) or the regex is invalid. The `not`
    /// modifier is carried into the matcher so [`StringMatcher::is_match`] applies it per
    /// value.
    pub fn string_matcher(self, expected: &str) -> Result<StringMatcher, BioAssertError> {
        let kind = match self.op {
            Operator::Eq => MatcherKind::Eq(expected.to_string()),
            Operator::Ne => MatcherKind::Ne(expected.to_string()),
            Operator::Starts => MatcherKind::Starts(expected.to_string()),
            Operator::Ends => MatcherKind::Ends(expected.to_string()),
            Operator::Contains => MatcherKind::Contains(expected.to_string()),
            Operator::Matches => MatcherKind::Matches(regex::Regex::new(expected)?),
            _ => {
                return Err(ComparatorError::UnsupportedComparator(format!(
                    "unsupported comparator for string comparison: {}",
                    self
                ))
                .into());
            }
        };
        Ok(StringMatcher {
            kind,
            negate: self.negate,
        })
    }
}

impl From<Operator> for Comparator {
    fn from(op: Operator) -> Self {
        Self::new(op)
    }
}

/// A reusable string predicate produced by [`Comparator::string_matcher`]. It owns its
/// expected value (and a compiled regex for `matches`) so [`Self::is_match`] can be
/// applied across many values without recompiling. This is the single source of truth
/// for string comparison semantics; [`Comparator::compare_string`] delegates to it. The
/// `negate` flag carries the `not` modifier, XORed into every match so the negation is
/// applied per value.
pub struct StringMatcher {
    kind: MatcherKind,
    negate: bool,
}

enum MatcherKind {
    Eq(String),
    Ne(String),
    Starts(String),
    Ends(String),
    Contains(String),
    Matches(regex::Regex),
}

impl StringMatcher {
    pub fn is_match(&self, actual: &str) -> bool {
        let base = match &self.kind {
            MatcherKind::Eq(expected) => actual == expected,
            MatcherKind::Ne(expected) => actual != expected,
            MatcherKind::Starts(expected) => actual.starts_with(expected.as_str()),
            MatcherKind::Ends(expected) => actual.ends_with(expected.as_str()),
            MatcherKind::Contains(expected) => actual.contains(expected.as_str()),
            MatcherKind::Matches(re) => re.is_match(actual),
        };
        base ^ self.negate
    }
}

impl FromStr for Operator {
    type Err = ComparatorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eq" => Ok(Self::Eq),
            "ne" => Ok(Self::Ne),
            "lt" => Ok(Self::Lt),
            "lte" => Ok(Self::Le),
            "gt" => Ok(Self::Gt),
            "gte" => Ok(Self::Ge),
            "starts" => Ok(Self::Starts),
            "ends" => Ok(Self::Ends),
            "contains" => Ok(Self::Contains),
            "matches" => Ok(Self::Matches),
            _ => Err(UnknownComparator(format!(
                "unknown comparator: {} (expected: eq, ne, lt, lte, gt, gte, starts, ends, contains, matches, each optionally prefixed with `not`)",
                s
            ))),
        }
    }
}

impl FromStr for Comparator {
    type Err = ComparatorError;

    /// Parses a comparator, splitting off an optional leading `not` (case-insensitive,
    /// followed by whitespace) into the `negate` flag. The grammar produces the whole
    /// token as one source slice (e.g. `not contains`), so this is the single place the
    /// modifier is interpreted.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let (negate, op_str) = match trimmed.split_once(char::is_whitespace) {
            Some((first, rest)) if first.eq_ignore_ascii_case("not") => (true, rest.trim_start()),
            _ => (false, trimmed),
        };
        Ok(Self {
            op: op_str.parse()?,
            negate,
        })
    }
}

impl Display for Comparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.negate {
            write!(f, "not ")?;
        }
        let s = match self.op {
            Operator::Eq => "==",
            Operator::Ne => "!=",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
            Operator::Starts => "starts_with",
            Operator::Ends => "ends_with",
            Operator::Contains => "contains",
            Operator::Matches => "matches",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-negated comparator from an operator, to keep the comparison tests terse.
    fn c(op: Operator) -> Comparator {
        Comparator::new(op)
    }

    #[test]
    fn from_str_parses_all_numeric() {
        assert_eq!("eq".parse::<Comparator>().unwrap(), c(Operator::Eq));
        assert_eq!("ne".parse::<Comparator>().unwrap(), c(Operator::Ne));
        assert_eq!("lt".parse::<Comparator>().unwrap(), c(Operator::Lt));
        assert_eq!("lte".parse::<Comparator>().unwrap(), c(Operator::Le));
        assert_eq!("gt".parse::<Comparator>().unwrap(), c(Operator::Gt));
        assert_eq!("gte".parse::<Comparator>().unwrap(), c(Operator::Ge));
    }

    #[test]
    fn from_str_parses_all_string() {
        assert_eq!("starts".parse::<Comparator>().unwrap(), c(Operator::Starts));
        assert_eq!("ends".parse::<Comparator>().unwrap(), c(Operator::Ends));
        assert_eq!(
            "contains".parse::<Comparator>().unwrap(),
            c(Operator::Contains)
        );
        assert_eq!(
            "matches".parse::<Comparator>().unwrap(),
            c(Operator::Matches)
        );
    }

    #[test]
    fn from_str_parses_a_negated_comparator() {
        let parsed = "not contains".parse::<Comparator>().unwrap();
        assert_eq!(parsed.op, Operator::Contains);
        assert!(parsed.negate);
    }

    #[test]
    fn from_str_negation_is_case_insensitive_in_the_prefix() {
        assert!("NOT matches".parse::<Comparator>().unwrap().negate);
        assert!("Not starts".parse::<Comparator>().unwrap().negate);
    }

    #[test]
    fn from_str_negates_a_numeric_comparator_too() {
        let parsed = "not gt".parse::<Comparator>().unwrap();
        assert_eq!(parsed.op, Operator::Gt);
        assert!(parsed.negate);
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!(matches!(
            "like".parse::<Comparator>(),
            Err(ComparatorError::UnknownComparator(_))
        ));
    }

    #[test]
    fn from_str_rejects_a_bare_not() {
        // `not` with no operator is not a comparator
        assert!("not".parse::<Comparator>().is_err());
    }

    #[test]
    fn compare_eq() {
        assert!(c(Operator::Eq).compare(5u64, 5u64));
        assert!(!c(Operator::Eq).compare(5u64, 6u64));
    }

    #[test]
    fn compare_ne() {
        assert!(c(Operator::Ne).compare(5u64, 6u64));
        assert!(!c(Operator::Ne).compare(5u64, 5u64));
    }

    #[test]
    fn compare_lt() {
        assert!(c(Operator::Lt).compare(4u64, 5u64));
        assert!(!c(Operator::Lt).compare(5u64, 5u64));
    }

    #[test]
    fn compare_le() {
        assert!(c(Operator::Le).compare(5u64, 5u64));
        assert!(c(Operator::Le).compare(4u64, 5u64));
        assert!(!c(Operator::Le).compare(6u64, 5u64));
    }

    #[test]
    fn compare_gt() {
        assert!(c(Operator::Gt).compare(6u64, 5u64));
        assert!(!c(Operator::Gt).compare(5u64, 5u64));
    }

    #[test]
    fn compare_ge() {
        assert!(c(Operator::Ge).compare(5u64, 5u64));
        assert!(c(Operator::Ge).compare(6u64, 5u64));
        assert!(!c(Operator::Ge).compare(4u64, 5u64));
    }

    #[test]
    fn compare_inverts_under_negation() {
        // `not gt` is the inverse of `gt`
        let not_gt = "not gt".parse::<Comparator>().unwrap();
        assert!(!not_gt.compare(6u64, 5u64));
        assert!(not_gt.compare(5u64, 5u64));
        assert!(not_gt.compare(4u64, 5u64));
    }

    #[test]
    fn compare_returns_false_for_string_operators_even_when_negated() {
        // A string comparator on a numeric/boolean metric is a mis-pairing. It must stay an
        // always-false (a visible FAIL), not flip to an always-true under `not`, which would
        // silently pass a check like `file.size not contains 1MB` in a validation gate.
        assert!(!c(Operator::Contains).compare(5u64, 5u64));
        assert!(
            !"not contains"
                .parse::<Comparator>()
                .unwrap()
                .compare(5u64, 5u64)
        );
        assert!(
            !"not matches"
                .parse::<Comparator>()
                .unwrap()
                .compare(5u64, 5u64)
        );
    }

    #[test]
    fn compare_string_eq() {
        assert!(c(Operator::Eq).compare_string("hello", "hello").unwrap());
        assert!(!c(Operator::Eq).compare_string("hello", "world").unwrap());
    }

    #[test]
    fn compare_string_ne() {
        assert!(c(Operator::Ne).compare_string("hello", "world").unwrap());
        assert!(!c(Operator::Ne).compare_string("hello", "hello").unwrap());
    }

    #[test]
    fn compare_string_starts() {
        assert!(c(Operator::Starts).compare_string("Alice", "Al").unwrap());
        assert!(!c(Operator::Starts).compare_string("Alice", "li").unwrap());
    }

    #[test]
    fn compare_string_ends() {
        assert!(c(Operator::Ends).compare_string("Alice", "ce").unwrap());
        assert!(!c(Operator::Ends).compare_string("Alice", "Al").unwrap());
    }

    #[test]
    fn compare_string_contains() {
        assert!(
            c(Operator::Contains)
                .compare_string("Alice", "lic")
                .unwrap()
        );
        assert!(
            !c(Operator::Contains)
                .compare_string("Alice", "xyz")
                .unwrap()
        );
    }

    #[test]
    fn compare_string_matches_valid_regex() {
        assert!(
            c(Operator::Matches)
                .compare_string("Alice", "^A.*e$")
                .unwrap()
        );
        assert!(!c(Operator::Matches).compare_string("Bob", "^A").unwrap());
    }

    #[test]
    fn compare_string_inverts_under_negation() {
        // `not contains` passes when the substring is absent
        let not_contains = "not contains".parse::<Comparator>().unwrap();
        assert!(not_contains.compare_string("Alice", "xyz").unwrap());
        assert!(!not_contains.compare_string("Alice", "lic").unwrap());
        // `not matches` gives regex negation without lookahead
        let not_matches = "not matches".parse::<Comparator>().unwrap();
        assert!(not_matches.compare_string("chr1", "^NC_").unwrap());
        assert!(!not_matches.compare_string("NC_001", "^NC_").unwrap());
    }

    #[test]
    fn string_matcher_bakes_in_negation_for_streaming_use() {
        // The matcher reused per cell by the whole-column metrics must itself be negated,
        // so a `.all not contains X` yields "no cell contains X".
        let matcher = "not contains"
            .parse::<Comparator>()
            .unwrap()
            .string_matcher("ERR")
            .unwrap();
        assert!(matcher.is_match("ok")); // no ERR -> passes
        assert!(!matcher.is_match("ERR_42")); // contains ERR -> fails
    }

    #[test]
    fn compare_string_matches_invalid_regex_returns_err() {
        assert!(
            c(Operator::Matches)
                .compare_string("Alice", "[invalid")
                .is_err()
        );
    }

    #[test]
    fn compare_string_numeric_comparator_returns_err() {
        assert!(c(Operator::Lt).compare_string("a", "b").is_err());
    }

    #[test]
    fn display_numeric_comparators() {
        assert_eq!(c(Operator::Eq).to_string(), "==");
        assert_eq!(c(Operator::Ne).to_string(), "!=");
        assert_eq!(c(Operator::Lt).to_string(), "<");
        assert_eq!(c(Operator::Le).to_string(), "<=");
        assert_eq!(c(Operator::Gt).to_string(), ">");
        assert_eq!(c(Operator::Ge).to_string(), ">=");
    }

    #[test]
    fn display_string_comparators() {
        assert_eq!(c(Operator::Starts).to_string(), "starts_with");
        assert_eq!(c(Operator::Ends).to_string(), "ends_with");
        assert_eq!(c(Operator::Contains).to_string(), "contains");
        assert_eq!(c(Operator::Matches).to_string(), "matches");
    }

    #[test]
    fn display_prefixes_not_when_negated() {
        assert_eq!(
            "not contains".parse::<Comparator>().unwrap().to_string(),
            "not contains"
        );
    }
}
