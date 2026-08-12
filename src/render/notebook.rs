use crate::file::{
    apply_content_options, inspect_file_with_options, markdown_fence_for, ContentOptions,
    InspectedFile,
};
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use std::path::Path;

pub(crate) fn render_notebook_file(
    path: &Path,
    max_size: u64,
    options: &ContentOptions,
) -> Result<Option<String>> {
    let inspection_options = ContentOptions::default();
    let text = match inspect_file_with_options(path, max_size, &inspection_options)? {
        InspectedFile::Text(text) => text,
        InspectedFile::Binary(_) | InspectedFile::TooLarge { .. } => return Ok(None),
    };
    let rendered = render_notebook(&text)
        .with_context(|| format!("failed to parse notebook {}", path.display()))?;
    Ok(Some(apply_content_options(rendered, options)))
}

fn render_notebook(input: &str) -> Result<String> {
    let notebook: Value = serde_json::from_str(input)?;
    let cells = notebook
        .get("cells")
        .and_then(Value::as_array)
        .context("notebook has no cells array")?;
    let language = notebook_language(&notebook);
    let mut rendered = String::new();

    for (index, cell) in cells.iter().enumerate() {
        let cell_type = cell
            .get("cell_type")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        rendered.push_str(&format!("### Cell {} ({cell_type})\n", index + 1));

        let source = joined_text(cell.get("source"));
        match cell_type {
            "markdown" => {
                rendered.push('\n');
                rendered.push_str(&source);
                ensure_newline(&mut rendered);
            }
            "code" => {
                rendered.push_str(&fenced_block(&source, &language));
                if let Some(outputs) = cell.get("outputs").and_then(Value::as_array) {
                    for output in outputs {
                        render_output(&mut rendered, output);
                    }
                }
            }
            "raw" => rendered.push_str(&fenced_block(&source, "text")),
            _ => rendered.push_str(&fenced_block(&source, "text")),
        }
        rendered.push('\n');
    }

    Ok(rendered.trim_end().to_string())
}

fn notebook_language(notebook: &Value) -> String {
    let language = notebook
        .pointer("/metadata/language_info/name")
        .and_then(Value::as_str)
        .or_else(|| {
            notebook
                .pointer("/metadata/kernelspec/language")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            notebook
                .pointer("/metadata/kernelspec/name")
                .and_then(Value::as_str)
                .filter(|name| name.to_ascii_lowercase().contains("python"))
                .map(|_| "python")
        })
        .unwrap_or("text");

    sanitize_language(language)
}

fn sanitize_language(language: &str) -> String {
    let normalized = language.trim().to_ascii_lowercase();
    if normalized.starts_with("python") {
        return "python".to_string();
    }

    normalized
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '#'))
        .collect()
}

fn render_output(rendered: &mut String, output: &Value) {
    match output
        .get("output_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "stream" => {
            let label = if output.get("name").and_then(Value::as_str) == Some("stderr") {
                "**Standard error:**"
            } else {
                "**Output:**"
            };
            push_labeled_block(
                rendered,
                label,
                &strip_ansi(&joined_text(output.get("text"))),
                "",
            );
        }
        "error" => {
            let name = output
                .get("ename")
                .and_then(Value::as_str)
                .unwrap_or("Error");
            let value = output
                .get("evalue")
                .and_then(Value::as_str)
                .unwrap_or_default();
            rendered.push_str("\n**Error:** ");
            rendered.push_str(name);
            if !value.is_empty() {
                rendered.push_str(": ");
                rendered.push_str(&strip_ansi(value));
            }
            rendered.push('\n');
        }
        "execute_result" | "display_data" | "update_display_data" => {
            render_rich_output(rendered, output)
        }
        _ => {}
    }
}

fn render_rich_output(rendered: &mut String, output: &Value) {
    let Some(data) = output.get("data").and_then(Value::as_object) else {
        return;
    };

    if let Some(markdown) = data.get("text/markdown") {
        let text = joined_text(Some(markdown));
        rendered.push_str("\n**Output:**\n\n");
        rendered.push_str(&text);
        ensure_newline(rendered);
    } else if let Some(text) = data.get("text/plain") {
        push_labeled_block(
            rendered,
            "**Output:**",
            &strip_ansi(&joined_text(Some(text))),
            "",
        );
    } else if let Some(json) = data.get("application/json") {
        let text = serde_json::to_string_pretty(json).unwrap_or_else(|_| json.to_string());
        push_labeled_block(rendered, "**Output:**", &text, "json");
    } else if let Some(html) = data.get("text/html") {
        push_labeled_block(rendered, "**Output:**", &joined_text(Some(html)), "html");
    } else if let Some(mime) = data.keys().find(|mime| mime.starts_with("image/")) {
        rendered.push_str(&format!("\n**Output:** [{mime} image omitted]\n"));
    }
}

fn push_labeled_block(rendered: &mut String, label: &str, text: &str, language: &str) {
    rendered.push('\n');
    rendered.push_str(label);
    rendered.push('\n');
    rendered.push_str(&fenced_block(text, language));
}

fn fenced_block(text: &str, language: &str) -> String {
    let fence = markdown_fence_for(text);
    let mut block = format!("\n{fence}{language}\n{text}");
    if !text.ends_with('\n') {
        block.push('\n');
    }
    block.push_str(&fence);
    block.push('\n');
    block
}

fn joined_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(Value::as_str).collect::<String>(),
        Some(value) if !value.is_null() => value.to_string(),
        _ => String::new(),
    }
}

fn strip_ansi(text: &str) -> String {
    Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]")
        .expect("ANSI escape regex should be valid")
        .replace_all(text, "")
        .into_owned()
}

fn ensure_newline(text: &mut String) {
    if !text.ends_with('\n') {
        text.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_code_stream_error_and_markdown_cells() {
        let notebook = r##"{
          "metadata": {"language_info": {"name": "python"}},
          "cells": [
            {
              "cell_type": "code",
              "source": ["print('Success')"],
              "outputs": [{"output_type": "stream", "name": "stdout", "text": ["Success\n"]}]
            },
            {
              "cell_type": "code",
              "source": ["raise FileNotFoundError('missing.xlsx')"],
              "outputs": [{
                "output_type": "error",
                "ename": "FileNotFoundError",
                "evalue": "No such file or directory: 'missing.xlsx'",
                "traceback": ["ignored"]
              }]
            },
            {"cell_type": "markdown", "source": ["# Analysis\n", "Some notes."]}
          ]
        }"##;

        let output = render_notebook(notebook).expect("notebook should render");

        assert!(output.contains("### Cell 1 (code)\n\n```python\nprint('Success')\n```"));
        assert!(output.contains("**Output:**\n\n```\nSuccess\n```"));
        assert!(output
            .contains("**Error:** FileNotFoundError: No such file or directory: 'missing.xlsx'"));
        assert!(output.contains("### Cell 3 (markdown)\n\n# Analysis\nSome notes."));
        assert!(!output.contains("ignored"));
    }

    #[test]
    fn renders_rich_outputs_and_uses_adaptive_fences() {
        let notebook = r#"{
          "metadata": {"kernelspec": {"language": "Julia"}},
          "cells": [{
            "cell_type": "code",
            "source": ["```"],
            "outputs": [
              {"output_type": "execute_result", "data": {"text/plain": ["42"]}},
              {"output_type": "display_data", "data": {"image/png": "abc"}}
            ]
          }]
        }"#;

        let output = render_notebook(notebook).expect("notebook should render");

        assert!(output.contains("````julia\n```\n````"));
        assert!(output.contains("**Output:**\n\n```\n42\n```"));
        assert!(output.contains("**Output:** [image/png image omitted]"));
    }
}
