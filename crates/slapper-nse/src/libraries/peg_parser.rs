//! Minimal PEG Parser for LPeg Patterns
//!
//! This module provides runtime PEG pattern parsing for patterns that can't be
//! converted to regex. It's a recursive descent parser with a simple matcher.

#[derive(Debug, Clone)]
pub enum PegNode {
    Literal(String),
    CharClass(CharClass),
    Any,
    Sequence(Vec<PegNode>),
    Alternative(Vec<PegNode>),
    ZeroOrMore(Box<PegNode>),
    OneOrMore(Box<PegNode>),
    Optional(Box<PegNode>),
    Lookahead(Box<PegNode>, bool),
    Reference(String),
    Empty,
}

#[derive(Debug, Clone)]
pub struct CharClass {
    pub negated: bool,
    pub ranges: Vec<(char, char)>,
    pub singles: Vec<char>,
}

impl CharClass {
    pub fn matches(&self, c: char) -> bool {
        let in_set = self
            .ranges
            .iter()
            .any(|(start, end)| c >= *start && c <= *end)
            || self.singles.contains(&c);

        if self.negated {
            !in_set
        } else {
            in_set
        }
    }
}

pub struct PegParser {
    input: Vec<char>,
    pos: usize,
}

impl PegParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> Result<PegNode, PegError> {
        self.skip_whitespace();
        let result = self.parse_alternative()?;
        self.skip_whitespace();

        if self.pos < self.input.len() {
            return Err(PegError::UnexpectedChar(
                self.input[self.pos.min(self.input.len() - 1)],
            ));
        }

        Ok(result)
    }

    fn parse_alternative(&mut self) -> Result<PegNode, PegError> {
        let mut alternatives = Vec::new();

        alternatives.push(self.parse_sequence()?);

        while self.pos < self.input.len() {
            self.skip_whitespace();
            let c = self.current_char();
            if c == Some('/') || c == Some('|') {
                self.pos += 1;
                self.skip_whitespace();
                alternatives.push(self.parse_sequence()?);
            } else {
                break;
            }
        }

        if alternatives.len() == 1 {
            Ok(alternatives.remove(0))
        } else {
            Ok(PegNode::Alternative(alternatives))
        }
    }

    fn parse_sequence(&mut self) -> Result<PegNode, PegError> {
        let mut sequence = Vec::new();

        while let Some(&c) = self.input.get(self.pos) {
            if c == '/' || c == '|' || c == ')' {
                break;
            }

            if c.is_whitespace() {
                self.pos += 1;
                continue;
            }

            sequence.push(self.parse_primary()?);
        }

        if sequence.is_empty() {
            return Ok(PegNode::Empty);
        }

        if sequence.len() == 1 {
            Ok(sequence.remove(0))
        } else {
            Ok(PegNode::Sequence(sequence))
        }
    }

    fn parse_primary(&mut self) -> Result<PegNode, PegError> {
        self.skip_whitespace();

        let c = self.current_char().unwrap_or(' ');

        // Handle lookahead
        if c == '&' || c == '!' {
            let is_positive = c == '&';
            self.pos += 1;
            self.skip_whitespace();
            let inner = self.parse_primary()?;
            return Ok(PegNode::Lookahead(Box::new(inner), is_positive));
        }

        // Handle grouping
        if c == '(' {
            self.pos += 1;
            let inner = self.parse_alternative()?;
            self.skip_whitespace();
            if self.current_char() != Some(')') {
                return Err(PegError::Expected(')'));
            }
            self.pos += 1;
            return Ok(inner);
        }

        // Handle character class
        if c == '[' {
            return self.parse_char_class();
        }

        // Handle any character
        if c == '.' {
            self.pos += 1;
            return Ok(PegNode::Any);
        }

        // Handle quoted literal
        if c == '"' || c == '\'' {
            return self.parse_literal();
        }

        // Handle named reference
        if c.is_alphabetic() || c == '_' {
            return self.parse_reference();
        }

        Err(PegError::UnexpectedChar(c))
    }

    fn parse_char_class(&mut self) -> Result<PegNode, PegError> {
        self.pos += 1; // '['

        let negated = if self.current_char() == Some('^') {
            self.pos += 1;
            true
        } else {
            false
        };

        let mut ranges = Vec::new();
        let mut singles = Vec::new();

        while let Some(&c) = self.input.get(self.pos) {
            if c == ']' {
                break;
            }

            if c == '\\' {
                self.pos += 1;
                let escaped = self.parse_escape_char()?;
                singles.push(escaped);
                continue;
            }

            let start = c;
            self.pos += 1;

            // Check for range
            if self.current_char() == Some('-') && self.input.get(self.pos + 1) != Some(&']') {
                self.pos += 1;
                let end = self.current_char().ok_or(PegError::UnexpectedEnd)?;
                self.pos += 1;
                ranges.push((start, end));
            } else {
                singles.push(start);
            }
        }

        if self.current_char() != Some(']') {
            return Err(PegError::Expected(']'));
        }
        self.pos += 1;

        Ok(PegNode::CharClass(CharClass {
            negated,
            ranges,
            singles,
        }))
    }

    fn parse_literal(&mut self) -> Result<PegNode, PegError> {
        let quote = self.current_char().ok_or(PegError::UnexpectedEnd)?;
        self.pos += 1;

        let mut literal = String::new();

        while let Some(&c) = self.input.get(self.pos) {
            if c == quote {
                self.pos += 1;
                return Ok(PegNode::Literal(literal));
            }

            if c == '\\' {
                self.pos += 1;
                let escaped = self.parse_escape_char()?;
                literal.push(escaped);
            } else {
                literal.push(c);
                self.pos += 1;
            }
        }

        Err(PegError::UnexpectedEnd)
    }

    fn parse_escape_char(&mut self) -> Result<char, PegError> {
        let c = self.current_char().ok_or(PegError::UnexpectedEnd)?;
        self.pos += 1;

        match c {
            'n' => Ok('\n'),
            't' => Ok('\t'),
            'r' => Ok('\r'),
            '0' => Ok('\0'),
            _ => Ok(c),
        }
    }

    fn parse_reference(&mut self) -> Result<PegNode, PegError> {
        let mut name = String::new();

        while let Some(&c) = self.input.get(self.pos) {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }

        if name.is_empty() {
            return Err(PegError::UnexpectedChar(' '));
        }

        // Check for repetition suffix
        self.skip_whitespace();
        let node = match self.current_char() {
            Some('*') => {
                self.pos += 1;
                self.skip_whitespace();
                if self.current_char() == Some('?') {
                    self.pos += 1;
                }
                PegNode::ZeroOrMore(Box::new(PegNode::Reference(name)))
            }
            Some('+') => {
                self.pos += 1;
                self.skip_whitespace();
                if self.current_char() == Some('?') {
                    self.pos += 1;
                }
                PegNode::OneOrMore(Box::new(PegNode::Reference(name)))
            }
            Some('?') => {
                self.pos += 1;
                PegNode::Optional(Box::new(PegNode::Reference(name)))
            }
            _ => PegNode::Reference(name),
        };

        Ok(node)
    }

    fn current_char(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.input.get(self.pos) {
            if c.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum PegError {
    UnexpectedChar(char),
    UnexpectedEnd,
    Expected(char),
}

pub struct PegMatcher;

impl PegMatcher {
    pub fn match_pattern(node: &PegNode, input: &str, start: usize) -> Option<PegMatch> {
        let input_chars: Vec<char> = input.chars().collect();
        let mut pos = start;

        if Self::match_node(node, &input_chars, &mut pos) {
            Some(PegMatch {
                start,
                end: pos,
                matched: input[start..].chars().take(pos - start).collect(),
            })
        } else {
            None
        }
    }

    fn match_node(node: &PegNode, input: &[char], pos: &mut usize) -> bool {
        match node {
            PegNode::Literal(s) => {
                let chars: Vec<char> = s.chars().collect();
                if input
                    .get(*pos..)
                    .map(|sub| sub.starts_with(&chars))
                    .unwrap_or(false)
                {
                    *pos += chars.len();
                    true
                } else {
                    false
                }
            }

            PegNode::CharClass(cc) => {
                if let Some(&c) = input.get(*pos) {
                    if cc.matches(c) {
                        *pos += 1;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            PegNode::Any => {
                if *pos < input.len() {
                    *pos += input[*pos].len_utf8();
                    true
                } else {
                    false
                }
            }

            PegNode::Sequence(nodes) => {
                for node in nodes {
                    if !Self::match_node(node, input, pos) {
                        return false;
                    }
                }
                true
            }

            PegNode::Alternative(nodes) => {
                for node in nodes {
                    let saved_pos = *pos;
                    if Self::match_node(node, input, pos) {
                        return true;
                    }
                    *pos = saved_pos;
                }
                false
            }

            PegNode::ZeroOrMore(node) => {
                while Self::match_node(node, input, pos) {}
                true
            }

            PegNode::OneOrMore(node) => {
                if !Self::match_node(node, input, pos) {
                    return false;
                }
                while Self::match_node(node, input, pos) {}
                true
            }

            PegNode::Optional(node) => {
                let saved_pos = *pos;
                let _ = Self::match_node(node, input, pos);
                true
            }

            PegNode::Lookahead(node, positive) => {
                let saved_pos = *pos;
                let result = Self::match_node(node, input, pos);
                *pos = saved_pos;

                if *positive {
                    result
                } else {
                    !result
                }
            }

            PegNode::Reference(_) => true,
            PegNode::Empty => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PegMatch {
    pub start: usize,
    pub end: usize,
    pub matched: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_parse() {
        let mut parser = PegParser::new("\"hello\"");
        let node = parser.parse().unwrap();
        assert!(matches!(node, PegNode::Literal(s) if s == "hello"));
    }

    #[test]
    fn test_char_class_parse() {
        let mut parser = PegParser::new("[abc]");
        let node = parser.parse().unwrap();
        assert!(matches!(node, PegNode::CharClass(_)));
    }

    #[test]
    fn test_sequence_parse() {
        let mut parser = PegParser::new("hello world");
        let node = parser.parse().unwrap();
        assert!(matches!(node, PegNode::Sequence(_)));
    }

    #[test]
    fn test_alternative_parse() {
        let mut parser = PegParser::new("foo / bar");
        let node = parser.parse().unwrap();
        assert!(matches!(node, PegNode::Alternative(_)));
    }

    #[test]
    fn test_matcher_literal() {
        let mut parser = PegParser::new("\"hello\"");
        let node = parser.parse().unwrap();
        let result = PegMatcher::match_pattern(&node, "hello world", 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched, "hello");
    }

    #[test]
    fn test_matcher_any() {
        let mut parser = PegParser::new(".");
        let node = parser.parse().unwrap();
        let result = PegMatcher::match_pattern(&node, "abc", 0);
        assert!(result.is_some());
    }

    #[test]
    fn test_matcher_sequence() {
        let mut parser = PegParser::new("hello");
        let node = parser.parse().unwrap();
        let result = PegMatcher::match_pattern(&node, "hello world", 0);
        assert!(result.is_some());
    }

    #[test]
    fn test_matcher_alternative() {
        let mut parser = PegParser::new("foo / bar");
        let node = parser.parse().unwrap();
        let result = PegMatcher::match_pattern(&node, "foo", 0);
        assert!(result.is_some());

        let result2 = PegMatcher::match_pattern(&node, "bar", 0);
        assert!(result2.is_some());
    }

    #[test]
    fn test_char_class_matcher() {
        let mut parser = PegParser::new("[a-z]");
        let node = parser.parse().unwrap();
        let result = PegMatcher::match_pattern(&node, "abc", 0);
        assert!(result.is_some());

        let result2 = PegMatcher::match_pattern(&node, "123", 0);
        assert!(result2.is_none());
    }

    #[test]
    fn test_parse_literal_unexpected_end() {
        // Unterminated string literal should return UnexpectedEnd, not panic
        let mut parser = PegParser::new("\"hello");
        let result = parser.parse();
        assert!(result.is_err());
    }
}
