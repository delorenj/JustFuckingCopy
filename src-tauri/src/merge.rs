#[derive(Clone, Copy, Debug, Default)]
pub enum MergeStrategy {
    #[default]
    Initial,
    OverlapDeduped,
    SequentialAppend,
}

impl MergeStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::OverlapDeduped => "overlap-deduped",
            Self::SequentialAppend => "sequential-append",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MergeOutcome {
    pub merged_text: String,
    pub strategy: MergeStrategy,
    pub overlap_lines: usize,
}

pub fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn append_text(existing: &str, incoming: &str) -> MergeOutcome {
    let cleaned_existing = normalize_text(existing);
    let cleaned_incoming = normalize_text(incoming);

    if cleaned_existing.is_empty() {
        return MergeOutcome {
            merged_text: cleaned_incoming,
            strategy: MergeStrategy::Initial,
            overlap_lines: 0,
        };
    }

    if cleaned_incoming.is_empty() {
        return MergeOutcome {
            merged_text: cleaned_existing,
            strategy: MergeStrategy::SequentialAppend,
            overlap_lines: 0,
        };
    }

    let existing_lines = cleaned_existing
        .lines()
        .map(canonical_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let incoming_lines = cleaned_incoming
        .lines()
        .map(canonical_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if let Some(overlap_lines) = find_overlap(&existing_lines, &incoming_lines) {
        let original_lines = cleaned_incoming.lines().collect::<Vec<_>>();
        let trimmed = if overlap_lines >= original_lines.len() {
            String::new()
        } else {
            original_lines[overlap_lines..].join("\n")
        };
        let merged_text = if trimmed.trim().is_empty() {
            cleaned_existing
        } else {
            format!("{cleaned_existing}\n{trimmed}")
        };

        return MergeOutcome {
            merged_text,
            strategy: MergeStrategy::OverlapDeduped,
            overlap_lines,
        };
    }

    MergeOutcome {
        merged_text: format!("{cleaned_existing}\n{cleaned_incoming}"),
        strategy: MergeStrategy::SequentialAppend,
        overlap_lines: 0,
    }
}

fn find_overlap(existing: &[String], incoming: &[String]) -> Option<usize> {
    let max_overlap = existing.len().min(incoming.len());

    for overlap in (1..=max_overlap).rev() {
        let existing_slice = &existing[existing.len() - overlap..];
        let incoming_slice = &incoming[..overlap];
        let average_score = existing_slice
            .iter()
            .zip(incoming_slice.iter())
            .map(|(left, right)| similarity(left, right))
            .sum::<f32>()
            / overlap as f32;

        let threshold = if overlap >= 3 { 0.78 } else { 0.93 };
        if average_score >= threshold {
            return Some(overlap);
        }
    }

    None
}

fn canonical_line(line: &str) -> String {
    line.to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn similarity(left: &str, right: &str) -> f32 {
    if left == right {
        return 1.0;
    }

    let distance = levenshtein(left, right) as f32;
    let max_len = left.chars().count().max(right.chars().count()) as f32;

    if max_len == 0.0 {
        1.0
    } else {
        1.0 - (distance / max_len)
    }
}

fn levenshtein(left: &str, right: &str) -> usize {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();

    if left.is_empty() {
        return right.len();
    }
    if right.is_empty() {
        return left.len();
    }

    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;

        for (right_index, right_char) in right.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current[right_index + 1] = (current[right_index] + 1)
                .min(previous[right_index + 1] + 1)
                .min(previous[right_index] + substitution_cost);
        }

        previous.clone_from_slice(&current);
    }

    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::{append_text, MergeStrategy};

    #[test]
    fn dedupes_line_overlap() {
        let first = "alpha\nbeta\ngamma";
        let second = "beta\ngamma\ndelta";
        let outcome = append_text(first, second);

        assert_eq!(outcome.strategy as u8, MergeStrategy::OverlapDeduped as u8);
        assert_eq!(outcome.overlap_lines, 2);
        assert_eq!(outcome.merged_text, "alpha\nbeta\ngamma\ndelta");
    }

    #[test]
    fn appends_sequentially_without_overlap() {
        let first = "alpha\nbeta";
        let second = "delta\nepsilon";
        let outcome = append_text(first, second);

        assert_eq!(
            outcome.strategy as u8,
            MergeStrategy::SequentialAppend as u8
        );
        assert_eq!(outcome.merged_text, "alpha\nbeta\ndelta\nepsilon");
    }
}
