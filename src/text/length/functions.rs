/// The length of a text resource, counted in Unicode scalar values (`char`s), not bytes,
/// so multibyte characters count as one each.
pub fn length(text: &str) -> u64 {
    text.chars().count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_ascii_characters() {
        assert_eq!(length("abc"), 3);
    }

    #[test]
    fn empty_string_is_zero() {
        assert_eq!(length(""), 0);
    }

    #[test]
    fn counts_unicode_scalars_not_bytes() {
        // "café" is 5 bytes but 4 Unicode scalar values.
        assert_eq!("café".len(), 5);
        assert_eq!(length("café"), 4);
    }
}
