use anyhow::Result;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

const MIME_DETECTION_BYTES: usize = 8 * 1024;
const MAX_CONTROL_BYTE_PERCENT: usize = 1;
const MIN_BINARY_CONTROL_BYTES: usize = 3;

#[derive(Debug, Clone)]
pub enum FileText {
    Text(String),
    Binary,
    TooLarge(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeDetection {
    Content,
    Extension,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub size: u64,
    pub mime_type: String,
    pub extension: Option<String>,
    pub mime_detection: MimeDetection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectedFile {
    Text(String),
    Binary(FileMetadata),
    TooLarge { metadata: FileMetadata, limit: u64 },
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
    Ok(match inspect_file_with_options(path, max_size, options)? {
        InspectedFile::Text(text) => FileText::Text(text),
        InspectedFile::Binary(_) => FileText::Binary,
        InspectedFile::TooLarge { limit, .. } => FileText::TooLarge(limit),
    })
}

pub fn inspect_file(path: &Path, max_size: u64, max_chars: Option<u64>) -> Result<InspectedFile> {
    inspect_file_with_options(
        path,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn inspect_file_with_options(
    path: &Path,
    max_size: u64,
    options: &ContentOptions,
) -> Result<InspectedFile> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MIME_DETECTION_BYTES as u64)
            .read_to_end(&mut bytes)?;
        return Ok(InspectedFile::TooLarge {
            metadata: detect_file_metadata(path, &bytes, metadata.len()),
            limit: max_size,
        });
    }

    let bytes = fs::read(path)?;
    let detection_bytes = &bytes[..bytes.len().min(MIME_DETECTION_BYTES)];

    if has_binary_content_signature(detection_bytes) || looks_binary(detection_bytes) {
        return Ok(InspectedFile::Binary(detect_file_metadata(
            path,
            detection_bytes,
            metadata.len(),
        )));
    }

    if let Some(text) = decode_utf16(&bytes) {
        return Ok(InspectedFile::Text(apply_content_options(text, options)));
    }

    match String::from_utf8(bytes) {
        Ok(text) => Ok(InspectedFile::Text(apply_content_options(text, options))),
        Err(error) => Ok(InspectedFile::Binary(detect_file_metadata(
            path,
            &error.as_bytes()[..error.as_bytes().len().min(MIME_DETECTION_BYTES)],
            metadata.len(),
        ))),
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
    match inspect_file_with_options(path, max_size, options)? {
        InspectedFile::Text(text) => Ok(text),
        InspectedFile::Binary(metadata) => Ok(format!(
            "[Binary file not shown; {}]",
            format_metadata(&metadata)
        )),
        InspectedFile::TooLarge { metadata, limit } => Ok(format!(
            "[File too large; {}; limit: {}]",
            format_metadata(&metadata),
            format_bytes(limit),
        )),
    }
}

pub(crate) fn format_metadata(metadata: &FileMetadata) -> String {
    let mut fields = vec![
        format!("MIME: {}", metadata.mime_type),
        format!(
            "detected by: {}",
            match metadata.mime_detection {
                MimeDetection::Content => "content signature",
                MimeDetection::Extension => "file extension",
                MimeDetection::Fallback => "fallback",
            }
        ),
        format!("size: {}", format_bytes(metadata.size)),
    ];
    if let Some(extension) = &metadata.extension {
        fields.push(format!("detected extension: {extension}"));
    }
    fields.join("; ")
}

fn has_binary_content_signature(bytes: &[u8]) -> bool {
    infer::get(bytes)
        .map(|kind| !is_textual_mime(kind.mime_type()))
        .unwrap_or(false)
}

fn is_textual_mime(mime_type: &str) -> bool {
    mime_type.starts_with("text/")
        || matches!(
            mime_type,
            "application/json"
                | "application/ld+json"
                | "application/javascript"
                | "application/postscript"
                | "application/rtf"
                | "application/xml"
                | "application/xhtml+xml"
                | "application/x-x509-ca-cert"
                | "image/svg+xml"
        )
        || mime_type.ends_with("+json")
        || mime_type.ends_with("+xml")
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() || has_utf16_bom(bytes) {
        return false;
    }
    if bytes.contains(&0) {
        return true;
    }

    let control_bytes = bytes
        .iter()
        .filter(|byte| matches!(byte, 0x01..=0x08 | 0x0b | 0x0e..=0x1f | 0x7f))
        .count();
    control_bytes >= MIN_BINARY_CONTROL_BYTES
        && control_bytes * 100 > bytes.len() * MAX_CONTROL_BYTE_PERCENT
}

fn has_utf16_bom(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff])
}

fn decode_utf16(bytes: &[u8]) -> Option<String> {
    let little_endian = if bytes.starts_with(&[0xff, 0xfe]) {
        true
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        false
    } else {
        return None;
    };

    let body = &bytes[2..];
    if !body.len().is_multiple_of(2) {
        return None;
    }
    let code_units = body.chunks_exact(2).map(|chunk| {
        let pair = [chunk[0], chunk[1]];
        if little_endian {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    });
    std::char::decode_utf16(code_units)
        .collect::<Result<String, _>>()
        .ok()
}

fn detect_file_metadata(path: &Path, bytes: &[u8], size: u64) -> FileMetadata {
    if let Some(kind) = infer::get(bytes) {
        return FileMetadata {
            size,
            mime_type: kind.mime_type().to_string(),
            extension: Some(kind.extension().to_string()),
            mime_detection: MimeDetection::Content,
        };
    }

    if let Some(mime_type) = mime_guess::from_path(path).first_raw() {
        return FileMetadata {
            size,
            mime_type: mime_type.to_string(),
            extension: path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase),
            mime_detection: MimeDetection::Extension,
        };
    }

    FileMetadata {
        size,
        mime_type: "application/octet-stream".to_string(),
        extension: None,
        mime_detection: MimeDetection::Fallback,
    }
}

pub(crate) fn format_bytes(size: u64) -> String {
    let digits = size.to_string();
    let mut formatted = String::new();
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(character);
    }
    let size = formatted.chars().rev().collect::<String>();
    let unit = if size == "1" { "byte" } else { "bytes" };
    format!("{size} {unit}")
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
    use std::fs;

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

    #[test]
    fn inspect_file_detects_binary_mime_type_and_metadata_from_content() {
        let root = std::env::temp_dir().join(format!("ccp-file-mime-test-{}", std::process::id()));
        let path = root.join("image.data");
        let png = [
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H',
            b'D', b'R',
        ];

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, png).expect("test image should be written");

        let inspected = inspect_file(&path, 1_000, None).expect("file should be inspected");

        assert_eq!(
            inspected,
            InspectedFile::Binary(FileMetadata {
                size: 16,
                mime_type: "image/png".to_string(),
                extension: Some("png".to_string()),
                mime_detection: MimeDetection::Content,
            })
        );
        assert!(matches!(
            read_file_text(&path, 1_000, None).expect("legacy read should succeed"),
            FileText::Binary
        ));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_rejects_valid_utf8_with_binary_control_bytes() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-control-test-{}", std::process::id()));
        let path = root.join("payload.bin");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, [0, 1, 2, 3, b'A']).expect("test binary should be written");

        assert_eq!(
            inspect_file(&path, 1_000, None).expect("file should be inspected"),
            InspectedFile::Binary(FileMetadata {
                size: 5,
                mime_type: "application/octet-stream".to_string(),
                extension: Some("bin".to_string()),
                mime_detection: MimeDetection::Extension,
            })
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_keeps_short_text_with_terminal_escapes() {
        let root = std::env::temp_dir().join(format!("ccp-file-ansi-test-{}", std::process::id()));
        let path = root.join("output.log");
        let text = "\x1b[31merror\x1b[0m\n";

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, text).expect("test text should be written");

        assert_eq!(
            inspect_file(&path, 1_000, None).expect("file should be inspected"),
            InspectedFile::Text(text.to_string())
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_uses_content_signature_before_utf8_classification() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-signature-test-{}", std::process::id()));
        let path = root.join("misleading.txt");
        let gif = b"GIF89a\x01\0\x01\0\x80\0\0\0\0\0\xff\xff\xff";

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, gif).expect("test image should be written");

        assert_eq!(
            inspect_file(&path, 1_000, None).expect("file should be inspected"),
            InspectedFile::Binary(FileMetadata {
                size: gif.len() as u64,
                mime_type: "image/gif".to_string(),
                extension: Some("gif".to_string()),
                mime_detection: MimeDetection::Content,
            })
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_keeps_textual_mime_signatures_as_text() {
        let root = std::env::temp_dir().join(format!("ccp-file-rtf-test-{}", std::process::id()));
        let path = root.join("document.rtf");
        let text = r"{\rtf1\ansi Hello from RTF}";

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, text).expect("test text should be written");

        assert_eq!(
            inspect_file(&path, 1_000, None).expect("file should be inspected"),
            InspectedFile::Text(text.to_string())
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_decodes_utf16_text_with_a_bom() {
        let root = std::env::temp_dir().join(format!("ccp-file-utf16-test-{}", std::process::id()));
        let path = root.join("notes.txt");
        let text = "Hello, UTF-16! 👋";
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, bytes).expect("test text should be written");

        assert_eq!(
            inspect_file(&path, 1_000, None).expect("file should be inspected"),
            InspectedFile::Text(text.to_string())
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_uses_extension_fallback_for_unknown_binary_data() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-extension-test-{}", std::process::id()));
        let path = root.join("archive.custom.pdf");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, [0xff, 0xfe, 0xfd]).expect("test binary should be written");

        let inspected = inspect_file(&path, 1_000, None).expect("file should be inspected");

        assert_eq!(
            inspected,
            InspectedFile::Binary(FileMetadata {
                size: 3,
                mime_type: "application/pdf".to_string(),
                extension: Some("pdf".to_string()),
                mime_detection: MimeDetection::Extension,
            })
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_falls_back_to_octet_stream_for_unknown_binary_data() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-fallback-test-{}", std::process::id()));
        let path = root.join("unknown");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, [0xff, 0xfe, 0xfd]).expect("test binary should be written");

        let inspected = inspect_file(&path, 1_000, None).expect("file should be inspected");

        assert_eq!(
            inspected,
            InspectedFile::Binary(FileMetadata {
                size: 3,
                mime_type: "application/octet-stream".to_string(),
                extension: None,
                mime_detection: MimeDetection::Fallback,
            })
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_keeps_valid_utf8_as_text_despite_binary_extension() {
        let root = std::env::temp_dir().join(format!("ccp-file-text-test-{}", std::process::id()));
        let path = root.join("notes.png");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, "plain text").expect("test text should be written");

        assert_eq!(
            inspect_file(&path, 1_000, None).expect("file should be inspected"),
            InspectedFile::Text("plain text".to_string())
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn inspect_file_reports_metadata_for_oversized_files() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-too-large-test-{}", std::process::id()));
        let path = root.join("large.png");
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        png.resize(20, 0);

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, png).expect("test image should be written");

        assert_eq!(
            inspect_file(&path, 10, None).expect("file should be inspected"),
            InspectedFile::TooLarge {
                metadata: FileMetadata {
                    size: 20,
                    mime_type: "image/png".to_string(),
                    extension: Some("png".to_string()),
                    mime_detection: MimeDetection::Content,
                },
                limit: 10,
            }
        );
        assert!(matches!(
            read_file_text(&path, 10, None).expect("legacy read should succeed"),
            FileText::TooLarge(10)
        ));

        fs::remove_dir_all(root).expect("test root should be removed");
    }
}
