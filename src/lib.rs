//! BioAssert - A bioinformatics assertion and validation library.
//!
//! This library provides functions for asserting and validating properties
//! of biological sequences and bioinformatics file formats.

use anyhow::{bail, Result};

/// Asserts that a DNA sequence contains only valid IUPAC nucleotide characters.
///
/// Valid characters are: A, T, G, C, N (case-insensitive).
///
/// # Errors
///
/// Returns an error if the sequence contains invalid characters.
///
/// # Examples
///
/// ```
/// use bioassert::assert_valid_dna;
///
/// assert!(assert_valid_dna("ATGCN").is_ok());
/// assert!(assert_valid_dna("ATGX").is_err());
/// ```
pub fn assert_valid_dna(sequence: &str) -> Result<()> {
    for (i, ch) in sequence.chars().enumerate() {
        match ch.to_ascii_uppercase() {
            'A' | 'T' | 'G' | 'C' | 'N' => {}
            invalid => bail!(
                "Invalid DNA character '{}' at position {} in sequence",
                invalid,
                i
            ),
        }
    }
    Ok(())
}

/// Asserts that an RNA sequence contains only valid IUPAC nucleotide characters.
///
/// Valid characters are: A, U, G, C, N (case-insensitive).
///
/// # Errors
///
/// Returns an error if the sequence contains invalid characters.
///
/// # Examples
///
/// ```
/// use bioassert::assert_valid_rna;
///
/// assert!(assert_valid_rna("AUGCN").is_ok());
/// assert!(assert_valid_rna("ATGC").is_err());
/// ```
pub fn assert_valid_rna(sequence: &str) -> Result<()> {
    for (i, ch) in sequence.chars().enumerate() {
        match ch.to_ascii_uppercase() {
            'A' | 'U' | 'G' | 'C' | 'N' => {}
            invalid => bail!(
                "Invalid RNA character '{}' at position {} in sequence",
                invalid,
                i
            ),
        }
    }
    Ok(())
}

/// Asserts that a protein sequence contains only valid amino acid characters.
///
/// Valid characters are the standard 20 amino acid single-letter codes plus X for unknown.
///
/// # Errors
///
/// Returns an error if the sequence contains invalid characters.
///
/// # Examples
///
/// ```
/// use bioassert::assert_valid_protein;
///
/// assert!(assert_valid_protein("ACDEFGHIKLMNPQRSTVWYX").is_ok());
/// assert!(assert_valid_protein("ACDB2").is_err());
/// ```
pub fn assert_valid_protein(sequence: &str) -> Result<()> {
    const VALID_AA: &str = "ACDEFGHIKLMNPQRSTVWYX*";
    for (i, ch) in sequence.chars().enumerate() {
        if !VALID_AA.contains(ch.to_ascii_uppercase()) {
            bail!(
                "Invalid amino acid character '{}' at position {} in sequence",
                ch,
                i
            );
        }
    }
    Ok(())
}

/// Asserts that a sequence length is within the specified range (inclusive).
///
/// # Errors
///
/// Returns an error if the length is outside the specified range.
///
/// # Examples
///
/// ```
/// use bioassert::assert_sequence_length;
///
/// assert!(assert_sequence_length("ATGC", 1, 10).is_ok());
/// assert!(assert_sequence_length("ATGC", 10, 20).is_err());
/// ```
pub fn assert_sequence_length(sequence: &str, min: usize, max: usize) -> Result<()> {
    let len = sequence.len();
    if len < min || len > max {
        bail!(
            "Sequence length {} is outside the expected range [{}, {}]",
            len,
            min,
            max
        );
    }
    Ok(())
}

/// Asserts that a sequence is non-empty.
///
/// # Errors
///
/// Returns an error if the sequence is empty.
///
/// # Examples
///
/// ```
/// use bioassert::assert_non_empty_sequence;
///
/// assert!(assert_non_empty_sequence("ATGC").is_ok());
/// assert!(assert_non_empty_sequence("").is_err());
/// ```
pub fn assert_non_empty_sequence(sequence: &str) -> Result<()> {
    if sequence.is_empty() {
        bail!("Sequence must not be empty");
    }
    Ok(())
}

/// Computes the GC content of a DNA or RNA sequence as a fraction between 0.0 and 1.0.
///
/// # Examples
///
/// ```
/// use bioassert::gc_content;
///
/// let gc = gc_content("ATGC");
/// assert!((gc - 0.5).abs() < f64::EPSILON);
/// ```
pub fn gc_content(sequence: &str) -> f64 {
    if sequence.is_empty() {
        return 0.0;
    }
    let gc_count = sequence
        .chars()
        .filter(|&c| matches!(c.to_ascii_uppercase(), 'G' | 'C'))
        .count();
    gc_count as f64 / sequence.len() as f64
}

/// Asserts that the GC content of a sequence is within the specified range (inclusive).
///
/// # Errors
///
/// Returns an error if the GC content is outside the specified range.
///
/// # Examples
///
/// ```
/// use bioassert::assert_gc_content;
///
/// assert!(assert_gc_content("ATGC", 0.4, 0.6).is_ok());
/// assert!(assert_gc_content("AAAA", 0.4, 0.6).is_err());
/// ```
pub fn assert_gc_content(sequence: &str, min: f64, max: f64) -> Result<()> {
    let gc = gc_content(sequence);
    if gc < min || gc > max {
        bail!(
            "GC content {:.4} is outside the expected range [{:.4}, {:.4}]",
            gc,
            min,
            max
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- assert_valid_dna ---

    #[test]
    fn test_valid_dna_uppercase() {
        assert!(assert_valid_dna("ATGCN").is_ok());
    }

    #[test]
    fn test_valid_dna_lowercase() {
        assert!(assert_valid_dna("atgcn").is_ok());
    }

    #[test]
    fn test_valid_dna_empty() {
        assert!(assert_valid_dna("").is_ok());
    }

    #[test]
    fn test_invalid_dna_contains_u() {
        let err = assert_valid_dna("ATGU").unwrap_err();
        assert!(err.to_string().contains('U'));
    }

    #[test]
    fn test_invalid_dna_contains_special_char() {
        assert!(assert_valid_dna("ATG!").is_err());
    }

    // --- assert_valid_rna ---

    #[test]
    fn test_valid_rna_uppercase() {
        assert!(assert_valid_rna("AUGCN").is_ok());
    }

    #[test]
    fn test_valid_rna_lowercase() {
        assert!(assert_valid_rna("augcn").is_ok());
    }

    #[test]
    fn test_invalid_rna_contains_t() {
        let err = assert_valid_rna("AUGCT").unwrap_err();
        assert!(err.to_string().contains('T'));
    }

    // --- assert_valid_protein ---

    #[test]
    fn test_valid_protein() {
        assert!(assert_valid_protein("ACDEFGHIKLMNPQRSTVWYX").is_ok());
    }

    #[test]
    fn test_valid_protein_stop_codon() {
        assert!(assert_valid_protein("MSTV*").is_ok());
    }

    #[test]
    fn test_invalid_protein() {
        assert!(assert_valid_protein("ACD2").is_err());
    }

    // --- assert_sequence_length ---

    #[test]
    fn test_sequence_length_in_range() {
        assert!(assert_sequence_length("ATGC", 1, 10).is_ok());
    }

    #[test]
    fn test_sequence_length_exact_min() {
        assert!(assert_sequence_length("ATGC", 4, 10).is_ok());
    }

    #[test]
    fn test_sequence_length_exact_max() {
        assert!(assert_sequence_length("ATGC", 1, 4).is_ok());
    }

    #[test]
    fn test_sequence_length_too_short() {
        assert!(assert_sequence_length("AT", 5, 10).is_err());
    }

    #[test]
    fn test_sequence_length_too_long() {
        assert!(assert_sequence_length("ATGCATGC", 1, 4).is_err());
    }

    // --- assert_non_empty_sequence ---

    #[test]
    fn test_non_empty_sequence_ok() {
        assert!(assert_non_empty_sequence("A").is_ok());
    }

    #[test]
    fn test_non_empty_sequence_err() {
        assert!(assert_non_empty_sequence("").is_err());
    }

    // --- gc_content ---

    #[test]
    fn test_gc_content_half() {
        assert!((gc_content("ATGC") - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gc_content_zero() {
        assert!((gc_content("AAAA") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gc_content_one() {
        assert!((gc_content("GCGC") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gc_content_empty() {
        assert!((gc_content("") - 0.0).abs() < f64::EPSILON);
    }

    // --- assert_gc_content ---

    #[test]
    fn test_assert_gc_content_in_range() {
        assert!(assert_gc_content("ATGC", 0.4, 0.6).is_ok());
    }

    #[test]
    fn test_assert_gc_content_too_low() {
        assert!(assert_gc_content("AAAA", 0.4, 0.6).is_err());
    }

    #[test]
    fn test_assert_gc_content_too_high() {
        assert!(assert_gc_content("GCGC", 0.4, 0.6).is_err());
    }
}
