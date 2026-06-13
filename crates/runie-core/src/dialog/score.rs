//! Fuzzy filtering score for panel items.

/// Score how well a label matches a query. Higher is better.
/// Priority: startsWith > contains > fuzzy variations.
pub fn match_score(label: &str, query: &str) -> Option<isize> {
    if query.is_empty() {
        return Some(0);
    }
    let label_lower = label.to_lowercase();
    let query_lower = query.to_lowercase();

    if label_lower.starts_with(&query_lower) {
        return Some(10_000 + (100 - label.len() as isize).max(0));
    }
    if label_lower.contains(&query_lower) {
        return Some(5_000 + (100 - label.len() as isize).max(0));
    }
    fuzzy_score(&label_lower, &query_lower)
}

fn fuzzy_score(label_lower: &str, query_lower: &str) -> Option<isize> {
    let label_chars: Vec<char> = label_lower.chars().collect();
    let query_chars: Vec<char> = query_lower.chars().collect();
    let mut q_idx = 0;
    let mut prev_match_idx: Option<usize> = None;
    let mut score: isize = 1_000;

    for (l_idx, l_ch) in label_chars.iter().enumerate() {
        if q_idx >= query_chars.len() {
            break;
        }
        if *l_ch == query_chars[q_idx] {
            if q_idx == 0 && l_idx == 0 {
                score += 50;
            }
            if let Some(prev) = prev_match_idx {
                let gap = l_idx - prev;
                if gap == 1 {
                    score += 20;
                } else {
                    score -= gap as isize * 5;
                }
            }
            prev_match_idx = Some(l_idx);
            q_idx += 1;
        }
    }

    if q_idx == query_chars.len() {
        Some(score)
    } else {
        None
    }
}
