pub fn fuzzy_score(text: &str, query: &str) -> u32 {
    let mut query_chars = query.chars().peekable();
    let mut score: u32 = 0;
    let mut last_match_idx: Option<usize> = None;
    let mut consecutive_bonus: u32 = 0;

    for (i, c) in text.chars().enumerate() {
        if let Some(&qc) = query_chars.peek() {
            if c.to_lowercase().next() == qc.to_lowercase().next() {
                query_chars.next();
                score += 1;
                if let Some(last) = last_match_idx {
                    if i == last + 1 {
                        consecutive_bonus += 2;
                    } else {
                        consecutive_bonus = 0;
                    }
                } else {
                    if i == 0 {
                        score += 3;
                    }
                }
                last_match_idx = Some(i);
            }
        }
    }

    if query_chars.peek().is_none() {
        score += consecutive_bonus;
        score
    } else {
        0
    }
}
