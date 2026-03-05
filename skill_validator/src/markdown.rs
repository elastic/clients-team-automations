use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct SectionData {
    pub level: i64,
    pub heading: String,
    pub line_number: i64,
    pub content: String,
    pub content_line_count: i64,
    pub code_blocks: Vec<CodeBlockData>,
}

#[derive(Debug, Clone)]
pub struct CodeBlockData {
    pub language: Option<String>,
    pub has_language_tag: bool,
    pub line_number: i64,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct MarkdownStructure {
    pub sections: Vec<SectionData>,
    pub has_title_heading: bool,
    pub title_heading: Option<String>,
    pub body_line_count: i64,
}

fn heading_level_to_i64(level: HeadingLevel) -> i64 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Pre-compute byte offsets of each newline so we can map any byte offset to a
/// 1-based line number with a binary search.
fn build_line_index(src: &str) -> Vec<usize> {
    src.bytes()
        .enumerate()
        .filter_map(|(i, b)| if b == b'\n' { Some(i) } else { None })
        .collect()
}

fn byte_offset_to_line(line_index: &[usize], offset: usize) -> usize {
    line_index.partition_point(|&nl| nl < offset) + 1
}

pub fn parse_markdown(body: &str, body_start_line: usize) -> MarkdownStructure {
    let body_line_count = body.lines().count() as i64;
    let line_index = build_line_index(body);

    let mut sections: Vec<SectionData> = Vec::new();
    let mut has_title_heading = false;
    let mut title_heading: Option<String> = None;

    let mut in_heading = false;
    let mut heading_level: i64 = 0;
    let mut heading_text = String::new();
    let mut heading_line: i64 = 0;

    let mut in_code_block = false;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_has_lang = false;
    let mut code_block_content = String::new();
    let mut code_block_line: i64 = 0;

    let mut pending_code_blocks: Vec<CodeBlockData> = Vec::new();
    // Track the byte range of content belonging to the current section
    // (everything after the heading line up to the next heading).
    let mut section_content_start: Option<usize> = None;
    let mut section_content_end: usize = 0;

    let parser = Parser::new_ext(body, Options::empty());

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = heading_level_to_i64(level);
                heading_text.clear();
                heading_line =
                    (body_start_line + byte_offset_to_line(&line_index, range.start) - 1) as i64;

                // Close previous section's content range
                if let Some(last) = sections.last_mut() {
                    let content = extract_section_content(
                        body,
                        section_content_start,
                        section_content_end,
                    );
                    last.content_line_count = content.lines().count() as i64;
                    last.content = content;
                    last.code_blocks = std::mem::take(&mut pending_code_blocks);
                }
                section_content_start = None;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let text = heading_text.trim().to_string();

                if heading_level == 1 && title_heading.is_none() {
                    has_title_heading = true;
                    title_heading = Some(text.clone());
                }

                sections.push(SectionData {
                    level: heading_level,
                    heading: text,
                    line_number: heading_line,
                    content: String::new(),
                    content_line_count: 0,
                    code_blocks: Vec::new(),
                });

                // Content for this section starts right after the heading
                section_content_start = Some(range.end);
                section_content_end = range.end;
            }
            Event::Text(t) if in_heading => {
                heading_text.push_str(&t);
            }
            Event::Code(t) if in_heading => {
                heading_text.push_str(&t);
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_content.clear();
                code_block_line =
                    (body_start_line + byte_offset_to_line(&line_index, range.start) - 1) as i64;
                match kind {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info.split_whitespace().next().unwrap_or("");
                        if lang.is_empty() {
                            code_block_lang = None;
                            code_block_has_lang = false;
                        } else {
                            code_block_lang = Some(lang.to_string());
                            code_block_has_lang = true;
                        }
                    }
                    CodeBlockKind::Indented => {
                        code_block_lang = None;
                        code_block_has_lang = false;
                    }
                }
                section_content_end = range.end;
            }
            Event::Text(t) if in_code_block => {
                code_block_content.push_str(&t);
                section_content_end = range.end;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                // pulldown-cmark includes a trailing newline in code block text; trim it
                let content = code_block_content
                    .strip_suffix('\n')
                    .unwrap_or(&code_block_content)
                    .to_string();
                pending_code_blocks.push(CodeBlockData {
                    language: code_block_lang.take(),
                    has_language_tag: code_block_has_lang,
                    line_number: code_block_line,
                    content,
                });
                section_content_end = range.end;
            }

            _ => {
                if !sections.is_empty() || section_content_start.is_some() {
                    section_content_end = range.end;
                }
            }
        }
    }

    // Close last section
    if let Some(last) = sections.last_mut() {
        let content =
            extract_section_content(body, section_content_start, section_content_end);
        last.content_line_count = content.lines().count() as i64;
        last.content = content;
        last.code_blocks = std::mem::take(&mut pending_code_blocks);
    }

    MarkdownStructure {
        sections,
        has_title_heading,
        title_heading,
        body_line_count,
    }
}

/// Extract the raw source text for a section's content (everything between the
/// heading line and the next heading or end-of-body).
fn extract_section_content(body: &str, start: Option<usize>, end: usize) -> String {
    match start {
        Some(s) if s < end && s < body.len() => {
            let slice = &body[s..end.min(body.len())];
            // The slice typically starts with a newline right after the heading;
            // trim exactly one leading newline to match the original behaviour
            // where the heading line itself was not included.
            let slice = slice.strip_prefix('\n').unwrap_or(slice);
            slice.to_string()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sections() {
        let body = "# Title\nIntro text\n## Examples\nSome examples\n## Guidelines\nSome guidelines";
        let result = parse_markdown(body, 5);
        assert!(result.has_title_heading);
        assert_eq!(result.title_heading.as_deref(), Some("Title"));
        assert_eq!(result.sections.len(), 3);
        assert_eq!(result.sections[1].heading, "Examples");
        assert_eq!(result.sections[2].heading, "Guidelines");
    }

    #[test]
    fn parse_code_blocks() {
        let body = "## Code\n```python\nprint('hello')\n```\n";
        let result = parse_markdown(body, 5);
        assert_eq!(result.sections.len(), 1);
        assert_eq!(result.sections[0].code_blocks.len(), 1);
        assert_eq!(
            result.sections[0].code_blocks[0].language.as_deref(),
            Some("python")
        );
        assert!(result.sections[0].code_blocks[0].has_language_tag);
    }

    #[test]
    fn code_block_without_language() {
        let body = "## Code\n```\nsome code\n```\n";
        let result = parse_markdown(body, 1);
        assert!(!result.sections[0].code_blocks[0].has_language_tag);
    }
}
