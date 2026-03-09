use rdx_ast::{Point, Position};

/// Maps byte offsets in the source document to 1-indexed line/column positions.
pub(crate) struct SourceMap {
    line_starts: Vec<usize>,
}

impl SourceMap {
    pub fn new(input: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (i, b) in input.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        SourceMap { line_starts }
    }

    pub fn point(&self, offset: usize) -> Point {
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        Point {
            line: line_idx + 1,
            column: offset - self.line_starts[line_idx] + 1,
            offset,
        }
    }

    pub fn position(&self, start: usize, end: usize) -> Position {
        Position {
            start: self.point(start),
            end: self.point(end),
        }
    }
}
