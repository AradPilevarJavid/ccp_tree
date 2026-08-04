use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum FileText {
    Text(String),
    Binary,
    TooLarge(u64),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ContentOptions {
    pub max_chars: Option<u64>,
    pub head: Option<usize>,
    pub tail: Option<usize>,
    pub from_end: bool,
}

pub fn read_file_text(path: &Path, max_size: u64, max_chars: Option<u64>) -> Result<FileText> {
    read_file_text_with_options(
        path,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn read_file_text_with_options(
    path: &Path,
    max_size: u64,
    options: &ContentOptions,
) -> Result<FileText> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Ok(FileText::TooLarge(max_size));
    }
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(FileText::Text(apply_content_options(text, options))),
        Err(_) => Ok(FileText::Binary),
    }
}

pub fn file_content(path: &Path, max_size: u64, max_chars: Option<u64>) -> Result<String> {
    file_content_with_options(
        path,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn file_content_with_options(
    path: &Path,
    max_size: u64,
    options: &ContentOptions,
) -> Result<String> {
    match read_file_text_with_options(path, max_size, options)? {
        FileText::Text(text) => Ok(text),
        FileText::Binary => Ok("[Binary file not shown]".to_string()),
        FileText::TooLarge(size) => Ok(format!("[File too large, > {} bytes]", size)),
    }
}

fn apply_content_options(mut text: String, options: &ContentOptions) -> String {
    if let Some(limit) = options.head {
        text = take_head_lines(&text, limit);
    } else if let Some(limit) = options.tail {
        text = take_tail_lines(&text, limit);
    }

    if let Some(limit) = options.max_chars {
        text = take_chars(&text, limit, options.from_end);
    }

    text
}

fn take_head_lines(text: &str, limit: usize) -> String {
    if text.lines().count() <= limit {
        return text.to_string();
    }

    let truncated = text.split_inclusive('\n').take(limit).collect::<String>();
    format!("{truncated}[truncated after {limit} lines]")
}

fn take_tail_lines(text: &str, limit: usize) -> String {
    if text.lines().count() <= limit {
        return text.to_string();
    }

    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let start = lines.len().saturating_sub(limit);
    let truncated = lines[start..].concat();
    format!("[truncated to last {limit} lines]\n{truncated}")
}

fn take_chars(text: &str, limit: u64, from_end: bool) -> String {
    let limit = limit as usize;
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_string();
    }

    if from_end {
        let truncated = text
            .chars()
            .skip(char_count.saturating_sub(limit))
            .collect::<String>();
        format!("[truncated to last {limit} characters]\n{truncated}")
    } else {
        let truncated = text.chars().take(limit).collect::<String>();
        format!("{truncated}\n[truncated after {limit} characters]")
    }
}

pub(crate) fn markdown_fence_for(content: &str) -> String {
    let mut max_run = 0;
    let mut current_run = 0;
    for character in content.chars() {
        if character == '`' {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    "`".repeat(std::cmp::max(3, max_run + 1))
}

pub(crate) fn is_single_line(text: &str) -> bool {
    !text.contains('\n') && !text.contains('\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_fence_is_longer_than_content_backtick_runs() {
        assert_eq!(markdown_fence_for("no fences"), "```");
        assert_eq!(markdown_fence_for("```rust\nfn main() {}\n```"), "````");
        assert_eq!(markdown_fence_for("````\ninner\n````"), "`````");
    }

    #[test]
    fn content_options_take_head_lines() {
        let text = apply_content_options(
            "one\ntwo\nthree\n".to_string(),
            &ContentOptions {
                head: Some(2),
                ..ContentOptions::default()
            },
        );

        assert_eq!(text, "one\ntwo\n[truncated after 2 lines]");
    }

    #[test]
    fn content_options_take_tail_lines() {
        let text = apply_content_options(
            "one\ntwo\nthree\n".to_string(),
            &ContentOptions {
                tail: Some(2),
                ..ContentOptions::default()
            },
        );

        assert_eq!(text, "[truncated to last 2 lines]\ntwo\nthree\n");
    }

    #[test]
    fn reverse_character_limit_takes_last_characters() {
        let text = apply_content_options(
            "abcdef".to_string(),
            &ContentOptions {
                max_chars: Some(3),
                from_end: true,
                ..ContentOptions::default()
            },
        );

        assert_eq!(text, "[truncated to last 3 characters]\ndef");
    }
}
