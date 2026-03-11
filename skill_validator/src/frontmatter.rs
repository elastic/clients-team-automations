use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub allowed_tools: Option<String>,
    pub metadata: BTreeMap<String, serde_yml::Value>,
    pub raw: String,
    pub begin_line: usize,
    pub end_line: usize,
    pub field_lines: BTreeMap<String, usize>,
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

    let mut field_lines: BTreeMap<String, usize> = BTreeMap::new();
    for (i, line) in lines.iter().enumerate().take(end_idx).skip(1) {
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if let Some(colon_pos) = line.find(':') {
                let key = &line[..colon_pos];
                if !key.is_empty() {
                    field_lines.insert(key.to_string(), i + 1); // 1-based file line
                }
            }
        }
    }

    let mut fm = Frontmatter {
        raw,
        begin_line,
        end_line,
        field_lines,
        ..Default::default()
    };

    if let Ok(serde_yml::Value::Mapping(map)) = yaml_value {
        for (k, v) in &map {
            let key = match k {
                serde_yml::Value::String(s) => s.clone(),
                _ => format!("{k:?}"),
            };

            match key.as_str() {
                "name" => fm.name = Some(yaml_value_to_string(v)),
                "description" => fm.description = Some(yaml_value_to_string(v)),
                "license" => fm.license = Some(yaml_value_to_string(v)),
                "compatibility" => fm.compatibility = Some(yaml_value_to_string(v)),
                "allowed-tools" | "allowed_tools" => fm.allowed_tools = Some(yaml_value_to_string(v)),
                _ => {
                    fm.metadata.insert(key, v.clone());
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

    #[test]
    fn parse_nested_metadata() {
        let content = "---\nname: my-skill\nmetadata:\n  version: 0.1.0\n  author: elastic\n---\n# Body";
        let result = parse_frontmatter(content);
        let fm = result.frontmatter.unwrap();
        assert_eq!(fm.name.as_deref(), Some("my-skill"));

        let meta_val = fm.metadata.get("metadata").expect("metadata key missing");
        assert!(meta_val.is_mapping(), "metadata value should be a YAML mapping");

        let map = meta_val.as_mapping().unwrap();
        let version = map.get(serde_yml::Value::String("version".into()));
        assert_eq!(
            version.and_then(|v| v.as_str()),
            Some("0.1.0"),
        );
        let author = map.get(serde_yml::Value::String("author".into()));
        assert_eq!(
            author.and_then(|v| v.as_str()),
            Some("elastic"),
        );
    }

    #[test]
    fn parse_scalar_metadata_preserved() {
        let content = "---\nname: my-skill\ncustom_key: hello\n---\n# Body";
        let result = parse_frontmatter(content);
        let fm = result.frontmatter.unwrap();
        let val = fm.metadata.get("custom_key").expect("custom_key missing");
        assert_eq!(val.as_str(), Some("hello"));
    }

    #[test]
    fn field_lines_tracks_name_line() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\n# Body";
        let result = parse_frontmatter(content);
        let fm = result.frontmatter.unwrap();
        assert_eq!(fm.field_lines.get("name").copied(), Some(2));
        assert_eq!(fm.field_lines.get("description").copied(), Some(3));
    }

    #[test]
    fn field_lines_skips_nested_keys() {
        let content = "---\nname: my-skill\nmetadata:\n  version: 0.1.0\n  author: elastic\n---\n# Body";
        let result = parse_frontmatter(content);
        let fm = result.frontmatter.unwrap();
        assert!(fm.field_lines.get("version").is_none(), "nested key should not be tracked");
        assert!(fm.field_lines.get("author").is_none(), "nested key should not be tracked");
        assert_eq!(fm.field_lines.get("metadata").copied(), Some(3));
    }

    #[test]
    fn field_lines_multiline_description() {
        let content = "---\nname: my-skill\ndescription: >\n  A multi-line\n  description here\n---\n# Body";
        let result = parse_frontmatter(content);
        let fm = result.frontmatter.unwrap();
        assert_eq!(fm.field_lines.get("description").copied(), Some(3));
    }
}
