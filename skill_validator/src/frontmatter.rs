use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub allowed_tools: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub raw: String,
    pub begin_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct FrontmatterResult {
    pub frontmatter: Option<Frontmatter>,
    pub body_start_line: usize,
}

pub fn parse_frontmatter(content: &str) -> FrontmatterResult {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return FrontmatterResult {
            frontmatter: None,
            body_start_line: 1,
        };
    }

    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let Some(end_idx) = end_idx else {
        return FrontmatterResult {
            frontmatter: None,
            body_start_line: 1,
        };
    };

    let raw: String = lines[1..end_idx].join("\n");
    let begin_line = 1; // 1-based, the opening ---
    let end_line = end_idx + 1; // 1-based, the closing ---
    let body_start_line = end_idx + 2; // 1-based, first line after closing ---

    let yaml_value: Result<serde_yml::Value, _> = serde_yml::from_str(&raw);

    let mut fm = Frontmatter {
        raw,
        begin_line,
        end_line,
        ..Default::default()
    };

    if let Ok(serde_yml::Value::Mapping(map)) = yaml_value {
        for (k, v) in &map {
            let key = match k {
                serde_yml::Value::String(s) => s.clone(),
                _ => format!("{k:?}"),
            };
            let value = yaml_value_to_string(v);

            match key.as_str() {
                "name" => fm.name = Some(value),
                "description" => fm.description = Some(value),
                "license" => fm.license = Some(value),
                "compatibility" => fm.compatibility = Some(value),
                "allowed-tools" | "allowed_tools" => fm.allowed_tools = Some(value),
                _ => {
                    fm.metadata.insert(key, value);
                }
            }
        }
    }

    FrontmatterResult {
        frontmatter: Some(fm),
        body_start_line,
    }
}

fn yaml_value_to_string(v: &serde_yml::Value) -> String {
    match v {
        serde_yml::Value::String(s) => s.clone(),
        serde_yml::Value::Bool(b) => b.to_string(),
        serde_yml::Value::Number(n) => n.to_string(),
        serde_yml::Value::Null => String::new(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\n# Body";
        let result = parse_frontmatter(content);
        let fm = result.frontmatter.unwrap();
        assert_eq!(fm.name.as_deref(), Some("my-skill"));
        assert_eq!(fm.description.as_deref(), Some("A test skill"));
        assert_eq!(result.body_start_line, 5);
    }

    #[test]
    fn parse_no_frontmatter() {
        let content = "# Just a heading\nSome content";
        let result = parse_frontmatter(content);
        assert!(result.frontmatter.is_none());
        assert_eq!(result.body_start_line, 1);
    }

    #[test]
    fn parse_unclosed_frontmatter() {
        let content = "---\nname: my-skill\nno closing";
        let result = parse_frontmatter(content);
        assert!(result.frontmatter.is_none());
    }
}
