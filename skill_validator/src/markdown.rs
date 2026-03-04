
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

pub fn parse_markdown(body: &str, body_start_line: usize) -> MarkdownStructure {
    let body_line_count = body.lines().count() as i64;

    let mut sections: Vec<SectionData> = Vec::new();
    let mut has_title_heading = false;
    let mut title_heading: Option<String> = None;

    let mut in_code_block = false;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();
    let mut code_block_start_line: i64 = 0;

    let mut pending_code_blocks: Vec<CodeBlockData> = Vec::new();
    let mut section_content_lines: Vec<String> = Vec::new();

    let lines: Vec<&str> = body.lines().collect();

    // Simple line-based parsing for headings and code blocks
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let abs_line = (body_start_line + i) as i64;

        if in_code_block {
            if line.starts_with("```") || line.starts_with("~~~") {
                pending_code_blocks.push(CodeBlockData {
                    language: code_block_lang.take(),
                    // Corrected by the fixup loop after parsing completes
                    has_language_tag: false,
                    line_number: code_block_start_line,
                    content: code_block_content.clone(),
                });
                code_block_content.clear();
                in_code_block = false;
            } else {
                if !code_block_content.is_empty() {
                    code_block_content.push('\n');
                }
                code_block_content.push_str(line);
            }
            section_content_lines.push(line.to_string());
            i += 1;
            continue;
        }

        if line.starts_with("```") || line.starts_with("~~~") {
            in_code_block = true;
            let fence = if line.starts_with("```") { "```" } else { "~~~" };
            let lang_tag = line[fence.len()..].trim();
            code_block_lang = if lang_tag.is_empty() {
                None
            } else {
                Some(lang_tag.to_string())
            };
            code_block_start_line = abs_line;
            code_block_content.clear();

            section_content_lines.push(line.to_string());
            i += 1;
            continue;
        }

        if line.starts_with('#') {
            let level = line.chars().take_while(|&c| c == '#').count() as i64;
            let text = line[level as usize..].trim().to_string();

            if level == 1 && title_heading.is_none() {
                has_title_heading = true;
                title_heading = Some(text.clone());
            }

            // Close current section if any
            if !sections.is_empty() || !section_content_lines.is_empty() {
                if let Some(last) = sections.last_mut() {
                    last.content = section_content_lines.join("\n");
                    last.content_line_count = section_content_lines.len() as i64;
                    last.code_blocks = std::mem::take(&mut pending_code_blocks);
                }
            }

            sections.push(SectionData {
                level,
                heading: text,
                line_number: abs_line,
                content: String::new(),
                content_line_count: 0,
                code_blocks: Vec::new(),
            });
            section_content_lines.clear();
            i += 1;
            continue;
        }

        section_content_lines.push(line.to_string());
        i += 1;
    }

    // Close last section
    if let Some(last) = sections.last_mut() {
        last.content = section_content_lines.join("\n");
        last.content_line_count = section_content_lines.len() as i64;
        last.code_blocks = std::mem::take(&mut pending_code_blocks);
    }

    // Handle unclosed code block
    if in_code_block {
        if let Some(last) = sections.last_mut() {
            last.code_blocks.push(CodeBlockData {
                language: code_block_lang,
                has_language_tag: false,
                line_number: code_block_start_line,
                content: code_block_content,
            });
        }
    }

    // Fix has_language_tag on all code blocks
    for section in &mut sections {
        for cb in &mut section.code_blocks {
            cb.has_language_tag = cb.language.is_some();
        }
    }

    MarkdownStructure {
        sections,
        has_title_heading,
        title_heading,
        body_line_count,
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
