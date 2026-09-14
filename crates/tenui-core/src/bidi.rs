//! Unicode Bidirectional (BiDi) Algorithm Text Reordering.

/// Checks if a character belongs to a strong Right-to-Left (RTL) script.
pub fn is_rtl_char(ch: char) -> bool {
    matches!(ch,
        '\u{0590}'..='\u{05FF}' // Hebrew
        | '\u{0600}'..='\u{06FF}' // Arabic
        | '\u{0700}'..='\u{074F}' // Syriac
        | '\u{0750}'..='\u{077F}' // Arabic Supplement
        | '\u{08A0}'..='\u{08FF}' // Arabic Extended-A
        | '\u{FB1D}'..='\u{FB4F}' // Hebrew Presentation Forms
        | '\u{FB50}'..='\u{FDFF}' // Arabic Presentation Forms-A
        | '\u{FE70}'..='\u{FEFF}' // Arabic Presentation Forms-B
    )
}

/// Checks if a string contains any RTL characters.
pub fn contains_rtl(s: &str) -> bool {
    s.chars().any(is_rtl_char)
}

/// Reorders a logical string into visual column order according to the Unicode Bidirectional Algorithm.
///
/// LTR segments are preserved left-to-right, while contiguous RTL runs are inverted
/// so they display properly in left-to-right character cell grids.
pub fn bidi_reorder(text: &str) -> String {
    if !contains_rtl(text) {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut current_rtl_run = Vec::new();

    for ch in text.chars() {
        if is_rtl_char(ch) {
            current_rtl_run.push(ch);
        } else {
            if !current_rtl_run.is_empty() {
                for &rc in current_rtl_run.iter().rev() {
                    result.push(rc);
                }
                current_rtl_run.clear();
            }
            result.push(ch);
        }
    }

    if !current_rtl_run.is_empty() {
        for &rc in current_rtl_run.iter().rev() {
            result.push(rc);
        }
    }

    result
}
