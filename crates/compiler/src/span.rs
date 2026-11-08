//! Source positions.

/// A byte range in the source text (`start..end`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// First byte (inclusive)
    pub start: usize,
    /// Last byte (exclusive)
    pub end: usize,
}

impl Span {
    /// Creates a span from byte offsets.
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

/// A human-readable position: 1-based line and column (in characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    /// 1-based line number
    pub line: usize,
    /// 1-based column, counted in characters
    pub col: usize,
}

/// Converts byte offsets to line/column positions.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Builds the index for `source`.
    pub fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(source.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex {
            source,
            line_starts,
        }
    }

    /// Line and column of a byte offset.
    pub fn line_col(&self, offset: usize) -> LineCol {
        let offset = offset.min(self.source.len());
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = self.line_starts[line];
        let col = self.source[start..offset].chars().count() + 1;
        LineCol {
            line: line + 1,
            col,
        }
    }

    /// Text of a 1-based line without its line break.
    pub fn line_text(&self, line: usize) -> &'a str {
        let start = self.line_starts[line - 1];
        let end = self
            .line_starts
            .get(line)
            .map(|&next| next - 1)
            .unwrap_or(self.source.len());
        self.source[start..end].trim_end_matches('\r')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_works() {
        let src = "ab\ncäd\n\nx";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(idx.line_col(3), LineCol { line: 2, col: 1 });
        // 'd' comes after the two-byte 'ä'
        assert_eq!(idx.line_col(6), LineCol { line: 2, col: 3 });
        assert_eq!(idx.line_col(9), LineCol { line: 4, col: 1 });
        assert_eq!(idx.line_text(2), "cäd");
        assert_eq!(idx.line_text(3), "");
        assert_eq!(idx.line_text(4), "x");
    }
}
