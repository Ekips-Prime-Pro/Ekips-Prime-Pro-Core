//! "Did you mean …?" suggestions for misspelled words.

use crate::keywords::TRANSLATIONS;

/// Edit distance with adjacent transpositions counted as one edit
/// (optimal string alignment), on characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m]
}

/// Finds the best replacement for `word` among `candidates`.
///
/// Known German or other-language words (e.g. `fahre`, `sleep`) are
/// translated first, as long as the translation is a candidate.
pub fn suggest(word: &str, candidates: &[&'static str]) -> Option<&'static str> {
    if let Some(&(_, english)) = TRANSLATIONS.iter().find(|(w, _)| *w == word) {
        if candidates.contains(&english) {
            return Some(english);
        }
    }
    let max = if word.chars().count() <= 4 { 1 } else { 2 };
    candidates
        .iter()
        .map(|&c| (edit_distance(word, c), c))
        .filter(|&(dist, _)| dist > 0 && dist <= max)
        .min_by_key(|&(dist, _)| dist)
        .map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance() {
        assert_eq!(edit_distance("drive", "drive"), 0);
        assert_eq!(edit_distance("drvie", "drive"), 1);
        assert_eq!(edit_distance("drve", "drive"), 1);
        assert_eq!(edit_distance("repaet", "repeat"), 1);
        assert_eq!(edit_distance("abc", ""), 3);
    }

    #[test]
    fn suggestions() {
        let kws = crate::keywords::STATEMENT_KEYWORDS;
        assert_eq!(suggest("cosnt", kws), Some("const"));
        assert_eq!(suggest("fucntion", kws), Some("function"));
        assert_eq!(suggest("funktion", kws), Some("function"));
        assert_eq!(suggest("wenn", kws), Some("if"));
        assert_eq!(suggest("banana", kws), None);
        // Translation target not among candidates: fall back to spelling.
        assert_eq!(suggest("wahr", &["if"]), None);
    }
}
