//! PEG Bridge Layer for LPeg
//!
//! Provides high-performance PEG pattern matching using a hybrid approach:
//! - Simple patterns: direct Rust string operations (fastest)
//! - Complex patterns: converted to regex for compatibility
//! - Pattern caching for repeated use

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::LazyLock;

static PATTERN_CACHE: LazyLock<DashMap<String, Arc<CompiledPattern>>> =
    LazyLock::new(|| DashMap::new());
static CACHE_MAX_SIZE: usize = 10_000;

pub struct CompiledPattern {
    pub pattern: String,
    pub pattern_type: PatternType,
    pub regex: Option<regex::Regex>,
    pub literal: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternType {
    Literal,
    CharSet,
    Range,
    Complex,
    Empty,
    Any,
}

impl CompiledPattern {
    pub fn new(pattern: &str) -> Self {
        let pattern_type = classify_pattern(pattern);

        let (regex, literal) = match &pattern_type {
            PatternType::Literal => (None, Some(pattern.to_string())),
            PatternType::CharSet | PatternType::Range => {
                let regex_pattern = lpeg_to_regex(pattern);
                let regex = regex::Regex::new(&regex_pattern).ok();
                (regex, None)
            }
            PatternType::Complex => {
                let regex_pattern = lpeg_to_regex(pattern);
                let regex = regex::Regex::new(&regex_pattern).ok();
                (regex, None)
            }
            PatternType::Empty => (None, Some(String::new())),
            PatternType::Any => (None, Some(".".to_string())),
        };

        Self {
            pattern: pattern.to_string(),
            pattern_type,
            regex,
            literal,
        }
    }

    pub fn from_cache(pattern: &str) -> Arc<Self> {
        let key = pattern.to_string();

        if let Some(cached) = PATTERN_CACHE.get(&key) {
            return Arc::clone(&cached);
        }

        let compiled = Arc::new(Self::new(pattern));

        if PATTERN_CACHE.len() >= CACHE_MAX_SIZE {
            // Simple eviction: clear half the cache when full
            let mut to_remove: Vec<String> = PATTERN_CACHE
                .iter()
                .take(CACHE_MAX_SIZE / 2)
                .map(|r| r.key().clone())
                .collect();
            for key in to_remove {
                PATTERN_CACHE.remove(&key);
            }
        }

        PATTERN_CACHE.insert(key, Arc::clone(&compiled));
        compiled
    }

    pub fn match_against(&self, text: &str) -> Option<MatchResult> {
        if let Some(ref literal) = self.literal {
            if literal == "." {
                // Match any single character
                if !text.is_empty() {
                    return Some(MatchResult {
                        start: 0,
                        end: 1,
                        matched: text.chars().next().unwrap().to_string(),
                    });
                }
                return None;
            }
            if let Some(pos) = text.find(literal) {
                return Some(MatchResult {
                    start: pos,
                    end: pos + literal.len(),
                    matched: literal.clone(),
                });
            }
            return None;
        }

        if let Some(ref re) = self.regex {
            if let Some(m) = re.find(text) {
                return Some(MatchResult {
                    start: m.start(),
                    end: m.end(),
                    matched: m.as_str().to_string(),
                });
            }
        }

        None
    }

    pub fn find_in(&self, text: &str, start: usize) -> Option<MatchResult> {
        if start >= text.len() {
            return None;
        }

        let search_text = &text[start..];

        if let Some(ref literal) = self.literal {
            if literal == "." {
                // Match any single character
                if !search_text.is_empty() {
                    let mut chars = search_text.chars();
                    let c = chars.next().unwrap();
                    return Some(MatchResult {
                        start,
                        end: start + c.len_utf8(),
                        matched: c.to_string(),
                    });
                }
                return None;
            }
            if let Some(pos) = search_text.find(literal) {
                return Some(MatchResult {
                    start: start + pos,
                    end: start + pos + literal.len(),
                    matched: literal.clone(),
                });
            }
            return None;
        }

        if let Some(ref re) = self.regex {
            if let Some(m) = re.find(search_text) {
                return Some(MatchResult {
                    start: start + m.start(),
                    end: start + m.end(),
                    matched: m.as_str().to_string(),
                });
            }
        }

        None
    }

    pub fn replace_all(&self, text: &str, replacement: &str) -> (String, usize) {
        if let Some(ref literal) = self.literal {
            if literal == "." {
                // Replace each character with replacement
                let count = text.chars().count();
                let result: String = text.chars().map(|_| replacement).collect();
                return (result, count);
            }
            let count = text.matches(literal).count();
            let result = text.replace(literal, replacement);
            return (result, count);
        }

        if let Some(ref re) = self.regex {
            let count = re.find_iter(text).count();
            let result = re.replace_all(text, replacement);
            return (result.to_string(), count);
        }

        (text.to_string(), 0)
    }

    /// Find all matches in the given text
    pub fn find_all(&self, text: &str) -> Vec<MatchResult> {
        let mut results = Vec::new();

        if let Some(ref literal) = self.literal {
            if literal == "." {
                // Match any single character
                for (i, c) in text.char_indices() {
                    results.push(MatchResult {
                        start: i,
                        end: i + c.len_utf8(),
                        matched: c.to_string(),
                    });
                }
                return results;
            }

            let mut start = 0;
            while let Some(pos) = text[start..].find(literal) {
                let absolute_pos = start + pos;
                results.push(MatchResult {
                    start: absolute_pos,
                    end: absolute_pos + literal.len(),
                    matched: literal.clone(),
                });
                start = absolute_pos + 1;
                if start >= text.len() {
                    break;
                }
            }
            return results;
        }

        if let Some(ref re) = self.regex {
            for m in re.find_iter(text) {
                results.push(MatchResult {
                    start: m.start(),
                    end: m.end(),
                    matched: m.as_str().to_string(),
                });
            }
        }

        results
    }
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub start: usize,
    pub end: usize,
    pub matched: String,
}

fn classify_pattern(pattern: &str) -> PatternType {
    let trimmed = pattern.trim();

    if trimmed.is_empty() {
        return PatternType::Empty;
    }

    if trimmed == "." {
        return PatternType::Any;
    }

    // Check if it's a simple literal (no special PEG characters)
    let has_peg_special = trimmed.chars().any(|c| {
        matches!(
            c,
            '(' | ')'
                | '*'
                | '+'
                | '?'
                | '['
                | ']'
                | '{'
                | '}'
                | '^'
                | '$'
                | '|'
                | '.'
                | '~'
                | '-'
                | ':'
                | '='
                | '_'
                | '@'
                | '#'
                | '!'
        )
    });

    if !has_peg_special {
        return PatternType::Literal;
    }

    // Check for character set patterns
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return PatternType::CharSet;
    }

    // Check for range patterns like "a-z" or "A-Z"
    if trimmed.contains("..") {
        return PatternType::Range;
    }

    // Check for simple character class abbreviations
    if trimmed == "\\d"
        || trimmed == "\\D"
        || trimmed == "\\w"
        || trimmed == "\\W"
        || trimmed == "\\s"
        || trimmed == "\\S"
    {
        return PatternType::CharSet;
    }

    PatternType::Complex
}

fn lpeg_to_regex(pattern: &str) -> String {
    let mut result = String::new();
    let mut chars = pattern.chars().peekable();
    let mut in_bracket = false;
    let mut i = 0;
    let bytes = pattern.as_bytes();

    while i < bytes.len() {
        let c = bytes[i] as char;

        match c {
            '[' => {
                in_bracket = true;
                result.push('[');
                // Check for negation
                if i + 1 < bytes.len() && bytes[i + 1] as char == '^' {
                    result.push('^');
                    i += 1;
                }
                // Check for ] as first character in set
                if i + 1 < bytes.len() && bytes[i + 1] as char == ']' {
                    result.push(']');
                    i += 1;
                }
            }
            ']' => {
                in_bracket = false;
                result.push(']');
            }
            '\\' => {
                if i + 1 < bytes.len() {
                    let next = bytes[i + 1] as char;
                    match next {
                        'd' => result.push_str(r"\d"),
                        'D' => result.push_str(r"\D"),
                        'w' => result.push_str(r"\w"),
                        'W' => result.push_str(r"\W"),
                        's' => result.push_str(r"\s"),
                        'S' => result.push_str(r"\S"),
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '0' => result.push('\0'),
                        // Handle escaped special chars
                        '(' | ')' | '*' | '+' | '?' | '[' | ']' | '{' | '}' | '^' | '$' | '|'
                        | '.' | '\\' | '/' | '-' | '=' => {
                            result.push(next);
                        }
                        _ => {
                            // Unknown escape, keep as-is
                            result.push('\\');
                            result.push(next);
                        }
                    }
                    i += 1;
                }
            }
            '-' => {
                if in_bracket {
                    result.push('-');
                } else if i > 0 && i < bytes.len() - 1 {
                    // Check if this looks like a range (alphanumeric on both sides)
                    let prev = bytes[i - 1] as char;
                    let next = bytes[i + 1] as char;
                    if prev.is_ascii_alphanumeric() && next.is_ascii_alphanumeric() {
                        result.push('-');
                    } else {
                        result.push_str(r"\-");
                    }
                } else {
                    result.push('-');
                }
            }
            '.' => {
                // In PEG, . matches any character
                // But in character class context, it's literal
                if in_bracket {
                    result.push('.');
                } else {
                    // Outside brackets, . is "any character"
                    // Only convert to .* if it's followed by * or + (zero or more, one or more)
                    if let Some(&next) = chars.peek() {
                        if next == '*' || next == '+' {
                            result.push('.');
                        } else {
                            // Single . in PEG = any char, but in regex . needs \. for literal
                            // For simplicity, treat . outside of quantifiers as any
                            result.push('.');
                        }
                    } else {
                        result.push('.');
                    }
                }
            }
            '^' => {
                // Only special at start of pattern or inside []
                if !in_bracket {
                    result.push('^');
                } else {
                    // Inside [] as first char, it's negation (handled above)
                    result.push('^');
                }
            }
            '$' => {
                // Only special at end of pattern
                result.push('$');
            }
            '*' => {
                // Zero or more - convert to *
                result.push('*');
                // Add ? for non-greedy if followed by ?
                if let Some(&next) = chars.peek() {
                    if next == '?' {
                        result.push('?');
                        chars.next();
                    }
                }
            }
            '+' => {
                // One or more
                result.push('+');
                if let Some(&next) = chars.peek() {
                    if next == '?' {
                        result.push('?');
                        chars.next();
                    }
                }
            }
            '?' => {
                // Optional
                result.push('?');
            }
            '|' => {
                // Alternation
                result.push('|');
            }
            '(' => {
                // Grouping
                result.push('(');
            }
            ')' => {
                result.push(')');
            }
            '{' => {
                // Quantifier {n} or {n,m}
                result.push('{');
            }
            '}' => {
                result.push('}');
            }
            '#' => {
                // Position capture - skip
                // But # may indicate non-capturing in some PEGs
            }
            '!' => {
                // Negative lookahead - not supported in regex, skip
            }
            '~' => {
                // Negation in some PEG variants
            }
            ':' | '=' => {
                // These are PEG-specific (ordered choice separator)
                // Just skip in conversion
            }
            _ => {
                if c.is_ascii_alphanumeric() || c == '_' || c == ' ' {
                    result.push(c);
                } else {
                    // Escape special regex characters
                    result.push('\\');
                    result.push(c);
                }
            }
        }
        i += 1;
    }

    if result.is_empty() {
        ".*".to_string()
    } else {
        result
    }
}

pub fn clear_cache() {
    PATTERN_CACHE.clear();
}

pub fn cache_size() -> usize {
    PATTERN_CACHE.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_pattern() {
        let pattern = CompiledPattern::from_cache("hello");
        let result = pattern.match_against("say hello world");
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched, "hello");
    }

    #[test]
    fn test_charset_pattern() {
        let pattern = CompiledPattern::from_cache("[a-z]+");
        let result = pattern.match_against("hello");
        assert!(result.is_some());
    }

    #[test]
    fn test_find() {
        let pattern = CompiledPattern::from_cache("world");
        let result = pattern.find_in("hello world", 6);
        assert!(result.is_some());
    }

    #[test]
    fn test_replace() {
        let pattern = CompiledPattern::from_cache("foo");
        let (result, count) = pattern.replace_all("foo bar foo", "baz");
        assert_eq!(result, "baz bar baz");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_any_pattern() {
        let pattern = CompiledPattern::from_cache(".");
        let result = pattern.match_against("abc");
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched, "a");
    }

    #[test]
    fn test_digit_class() {
        let pattern = CompiledPattern::from_cache("\\d+");
        let result = pattern.match_against("abc123def");
        assert!(result.is_some());
    }

    #[test]
    fn test_word_class() {
        let pattern = CompiledPattern::from_cache("\\w+");
        let result = pattern.match_against("hello_world");
        assert!(result.is_some());
    }

    #[test]
    fn test_whitespace_class() {
        let pattern = CompiledPattern::from_cache("\\s+");
        let result = pattern.match_against("hello world");
        assert!(result.is_some());
    }

    #[test]
    fn test_negated_charset() {
        let pattern = CompiledPattern::from_cache("[^0-9]");
        let result = pattern.match_against("123abc");
        assert!(result.is_some());
    }

    #[test]
    fn test_range() {
        let pattern = CompiledPattern::from_cache("[a-z]");
        let result = pattern.match_against("abc");
        assert!(result.is_some());
    }

    #[test]
    fn test_alternation() {
        let pattern = CompiledPattern::from_cache("foo|bar");
        let result = pattern.match_against("foo");
        assert!(result.is_some());
    }

    #[test]
    fn test_grouping() {
        let pattern = CompiledPattern::from_cache("(foo)+");
        let result = pattern.match_against("foofoo");
        assert!(result.is_some());
    }

    #[test]
    fn test_optional() {
        let pattern = CompiledPattern::from_cache("colou?r");
        let result = pattern.match_against("color");
        assert!(result.is_some());
        let result2 = pattern.match_against("colour");
        assert!(result2.is_some());
    }

    #[test]
    fn test_cache_eviction() {
        clear_cache();

        // Fill cache beyond max
        for i in 0..15000 {
            let _ = CompiledPattern::from_cache(&format!("pattern{}", i));
        }

        // Cache should have been pruned
        assert!(cache_size() <= CACHE_MAX_SIZE);
    }
}
