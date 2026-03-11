use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use walkdir::WalkDir;

use crate::config::Config;
use crate::frontmatter;
use crate::markdown::{self, SectionData};
use crate::references::{self, ReferencedPathData};

// ---------------------------------------------------------------------------
// Data types (each one backs a schema type)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SpanData {
    pub filename: String,
    pub begin_line: i64,
    pub end_line: i64,
}

#[derive(Debug, Clone)]
pub struct MetadataEntryData {
    pub key: String,
    pub value: String,
    pub children: Vec<Arc<MetadataEntryData>>,
}

const MAX_FILE_READ_BYTES: u64 = 1_048_576; // 1 MB

#[derive(Debug, Clone)]
pub struct SubDirFileData {
    pub name: String,
    pub extension: String,
    pub path: String,
    pub is_data_file: bool,
    pub content: Option<String>,
    pub referenced_paths: Vec<Arc<ReferencedPathData>>,
}

#[derive(Debug, Clone)]
pub struct SubDirData {
    pub name: String,
    pub path: String,
    pub files: Vec<Arc<SubDirFileData>>,
    pub file_count: i64,
    pub unique_extensions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillData {
    pub folder_name: String,
    pub group_folder: String,
    pub path: String,
    pub skill_file_path: String,
    pub depth: i64,

    pub has_frontmatter: bool,
    pub raw_frontmatter: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub allowed_tools: Option<String>,
    pub description_length: i64,
    pub description_word_count: i64,

    pub total_line_count: i64,
    pub body_line_count: i64,
    pub has_title_heading: bool,
    pub title_heading: Option<String>,

    pub metadata: Vec<Arc<MetadataEntryData>>,
    pub sections: Vec<Arc<SectionData>>,
    pub sub_dirs: Vec<Arc<SubDirData>>,
    pub root_files: Vec<Arc<SubDirFileData>>,
    pub referenced_paths: Vec<Arc<ReferencedPathData>>,
    pub span: Arc<SpanData>,
    pub frontmatter_span: Option<Arc<SpanData>>,
}

#[derive(Debug, Clone)]
pub struct GroupFolderData {
    pub name: String,
    pub path: String,
    pub skill_indices: Vec<usize>,
    pub skill_count: i64,
}

#[derive(Debug, Clone)]
pub struct DiscoveredSkillFileData {
    pub path: String,
    pub parent_dir: String,
    pub depth: i64,
    pub skill_index: Option<usize>,
    pub span: Arc<SpanData>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredDirectoryData {
    pub name: String,
    pub path: String,
    pub depth: i64,
    pub has_skill_file: bool,
    pub skill_index: Option<usize>,
    pub file_count: i64,
    pub files: Vec<Arc<SubDirFileData>>,
    pub span: Arc<SpanData>,
}

#[derive(Debug, Clone)]
pub struct GitHubTeamData {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitHubOrgData {
    pub name: String,
    pub teams_loaded: bool,
    pub teams: Vec<Arc<GitHubTeamData>>,
}

#[derive(Debug, Clone)]
pub struct SkillsData {
    pub skills: Vec<Arc<SkillData>>,
    pub group_folders: Vec<Arc<GroupFolderData>>,
    pub discovered_files: Vec<Arc<DiscoveredSkillFileData>>,
    pub discovered_dirs: Vec<Arc<DiscoveredDirectoryData>>,
    pub github_org: Arc<GitHubOrgData>,
}

// ---------------------------------------------------------------------------
// YAML → MetadataEntryData conversion
// ---------------------------------------------------------------------------

fn yaml_value_to_string(v: &serde_yml::Value) -> String {
    match v {
        serde_yml::Value::String(s) => s.clone(),
        serde_yml::Value::Bool(b) => b.to_string(),
        serde_yml::Value::Number(n) => n.to_string(),
        serde_yml::Value::Null => String::new(),
        other => format!("{other:?}"),
    }
}

fn yaml_value_to_metadata(key: String, value: &serde_yml::Value) -> MetadataEntryData {
    match value {
        serde_yml::Value::Mapping(map) => {
            let children: Vec<Arc<MetadataEntryData>> = map
                .iter()
                .map(|(k, v)| {
                    let child_key = match k {
                        serde_yml::Value::String(s) => s.clone(),
                        other => format!("{other:?}"),
                    };
                    Arc::new(yaml_value_to_metadata(child_key, v))
                })
                .collect();
            MetadataEntryData {
                key,
                value: yaml_value_to_string(value),
                children,
            }
        }
        _ => MetadataEntryData {
            key,
            value: yaml_value_to_string(value),
            children: vec![],
        },
    }
}

// ---------------------------------------------------------------------------
// Directory walking & data loading
// ---------------------------------------------------------------------------

pub fn load_skills_data(
    skills_dir: &Path,
    repo_root: &Path,
    config: &Config,
    scope_filter: Option<&std::collections::HashSet<String>>,
) -> SkillsData {
    let mut discovered_files: Vec<Arc<DiscoveredSkillFileData>> = Vec::new();
    let mut skills: Vec<Arc<SkillData>> = Vec::new();
    let mut group_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    // Walk the skills directory looking for SKILL.md files
    let skills_dir_abs = if skills_dir.is_absolute() {
        skills_dir.to_path_buf()
    } else {
        repo_root.join(skills_dir)
    };

    if !skills_dir_abs.exists() {
        return SkillsData {
            skills,
            group_folders: Vec::new(),
            discovered_files,
            discovered_dirs: Vec::new(),
            github_org: Arc::new(GitHubOrgData {
                name: String::new(),
                teams_loaded: false,
                teams: Vec::new(),
            }),
        };
    }

    for entry in WalkDir::new(&skills_dir_abs)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() != "SKILL.md" {
            continue;
        }

        let abs_path = entry.path();
        let rel_path = abs_path
            .strip_prefix(repo_root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .to_string();

        let skill_dir = abs_path.parent().unwrap_or(abs_path);
        let rel_skill_dir = skill_dir
            .strip_prefix(repo_root)
            .unwrap_or(skill_dir)
            .to_string_lossy()
            .to_string();

        // Compute depth: number of components under skills_dir
        let skills_rel = abs_path
            .strip_prefix(&skills_dir_abs)
            .unwrap_or(abs_path);
        let depth = skills_rel.components().count() as i64;
        let is_valid_location = depth >= 3;

        let parent_dir = skill_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let in_scope = match scope_filter {
            Some(filter) => filter.contains(&rel_skill_dir),
            None => true,
        };

        let content = match fs_err::read_to_string(abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let total_line_count = content.lines().count() as i64;

        let span = Arc::new(SpanData {
            filename: rel_path.clone(),
            begin_line: 1,
            end_line: total_line_count,
        });

        let skill_index = if is_valid_location && in_scope {

            let fm_result = frontmatter::parse_frontmatter(&content);
            let body = if fm_result.body_start_line > 1 {
                content
                    .lines()
                    .skip(fm_result.body_start_line - 1)
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                content.clone()
            };

            let md = markdown::parse_markdown(&body, fm_result.body_start_line);

            let folder_name = parent_dir.clone();
            let group_folder = skill_dir
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let (has_fm, raw_fm, name, description, license, compatibility, allowed_tools, metadata_entries, fm_span) =
                match &fm_result.frontmatter {
                    Some(fm) => {
                        let entries: Vec<Arc<MetadataEntryData>> = fm
                            .metadata
                            .iter()
                            .map(|(k, v)| Arc::new(yaml_value_to_metadata(k.clone(), v)))
                            .collect();
                        let span = Arc::new(SpanData {
                            filename: rel_path.clone(),
                            begin_line: fm.begin_line as i64,
                            end_line: fm.end_line as i64,
                        });
                        (
                            true,
                            Some(fm.raw.clone()),
                            fm.name.clone(),
                            fm.description.clone(),
                            fm.license.clone(),
                            fm.compatibility.clone(),
                            fm.allowed_tools.clone(),
                            entries,
                            Some(span),
                        )
                    }
                    None => (false, None, None, None, None, None, None, Vec::new(), None),
                };

            let desc_len = description.as_ref().map(|d| d.len() as i64).unwrap_or(0);
            let desc_words = description
                .as_ref()
                .map(|d| d.split_whitespace().count() as i64)
                .unwrap_or(0);

            let sections: Vec<Arc<SectionData>> =
                md.sections.into_iter().map(Arc::new).collect();

            let skill_refs: Vec<Arc<ReferencedPathData>> =
                references::referenced_paths_from_markdown_links(&md.links, &rel_path)
                    .into_iter()
                    .map(Arc::new)
                    .collect();

            let sub_dirs = scan_subdirs(skill_dir, repo_root, config);
            let root_files = scan_root_files(skill_dir, repo_root, config);

            let idx = skills.len();
            skills.push(Arc::new(SkillData {
                folder_name,
                group_folder: group_folder.clone(),
                path: rel_skill_dir.clone(),
                skill_file_path: rel_path.clone(),
                depth,
                has_frontmatter: has_fm,
                raw_frontmatter: raw_fm,
                name,
                description,
                license,
                compatibility,
                allowed_tools,
                description_length: desc_len,
                description_word_count: desc_words,
                total_line_count,
                body_line_count: md.body_line_count,
                has_title_heading: md.has_title_heading,
                title_heading: md.title_heading,
                metadata: metadata_entries,
                sections,
                sub_dirs,
                root_files,
                referenced_paths: skill_refs,
                span: span.clone(),
                frontmatter_span: fm_span,
            }));

            group_map
                .entry(group_folder)
                .or_default()
                .push(idx);

            Some(idx)
        } else {
            None
        };

        if in_scope {
            discovered_files.push(Arc::new(DiscoveredSkillFileData {
                path: rel_path,
                parent_dir,
                depth,
                skill_index,
                span,
            }));
        }
    }

    let group_folders: Vec<Arc<GroupFolderData>> = group_map
        .into_iter()
        .map(|(name, indices)| {
            let path = format!(
                "{}",
                skills_dir.join(&name).to_string_lossy()
            );
            let count = indices.len() as i64;
            Arc::new(GroupFolderData {
                name,
                path,
                skill_indices: indices,
                skill_count: count,
            })
        })
        .collect();

    // Build a lookup from skill directory path to index for cross-referencing
    let skill_path_to_index: BTreeMap<String, usize> = skills
        .iter()
        .enumerate()
        .map(|(i, s)| (s.path.clone(), i))
        .collect();

    let discovered_dirs = discover_directories(
        &skills_dir_abs,
        repo_root,
        config,
        &skill_path_to_index,
    );

    SkillsData {
        skills,
        group_folders,
        discovered_files,
        discovered_dirs,
        github_org: Arc::new(GitHubOrgData {
            name: String::new(),
            teams_loaded: false,
            teams: Vec::new(),
        }),
    }
}

/// Walk all directories at depth >= 2 under `skills_dir_abs` (the expected skill
/// directory depth) and build `DiscoveredDirectoryData` entries regardless of
/// whether each directory contains a SKILL.md.
fn discover_directories(
    skills_dir_abs: &Path,
    repo_root: &Path,
    config: &Config,
    skill_path_to_index: &BTreeMap<String, usize>,
) -> Vec<Arc<DiscoveredDirectoryData>> {
    let mut result = Vec::new();

    for entry in WalkDir::new(skills_dir_abs)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_dir() {
            continue;
        }

        let abs_path = entry.path();
        let dir_rel = abs_path
            .strip_prefix(skills_dir_abs)
            .unwrap_or(abs_path);
        let depth = dir_rel.components().count() as i64;

        if depth < 2 {
            continue;
        }

        let dir_name = abs_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let rel_path = abs_path
            .strip_prefix(repo_root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .to_string();

        let has_skill_file = abs_path.join("SKILL.md").is_file();
        let skill_index = skill_path_to_index.get(&rel_path).copied();

        let mut files: Vec<Arc<SubDirFileData>> = Vec::new();
        if let Ok(dir_entries) = std::fs::read_dir(abs_path) {
            for file_entry in dir_entries.filter_map(|e| e.ok()) {
                let file_path = file_entry.path();
                if !file_path.is_file() || file_path.file_name().is_some_and(|n| n == "SKILL.md") {
                    continue;
                }
                let file_name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let ext = file_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default();
                let rel_file = file_path
                    .strip_prefix(repo_root)
                    .unwrap_or(&file_path)
                    .to_string_lossy()
                    .to_string();

                let is_data = config.is_data_extension(&ext);
                let (file_content, file_refs) = read_and_extract(&file_path, &ext, &rel_file);

                files.push(Arc::new(SubDirFileData {
                    name: file_name,
                    extension: ext,
                    path: rel_file,
                    is_data_file: is_data,
                    content: file_content,
                    referenced_paths: file_refs,
                }));
            }
        }

        let file_count = files.len() as i64;
        let span = Arc::new(SpanData {
            filename: rel_path.clone(),
            begin_line: 1,
            end_line: 1,
        });

        result.push(Arc::new(DiscoveredDirectoryData {
            name: dir_name,
            path: rel_path,
            depth,
            has_skill_file,
            skill_index,
            file_count,
            files,
            span,
        }));
    }

    result
}

/// Read a file's text content (if small enough and valid UTF-8) and extract
/// any local path references from it.
fn read_and_extract(
    abs_path: &Path,
    extension: &str,
    rel_path: &str,
) -> (Option<String>, Vec<Arc<ReferencedPathData>>) {
    let meta = match std::fs::metadata(abs_path) {
        Ok(m) => m,
        Err(_) => return (None, Vec::new()),
    };
    if meta.len() > MAX_FILE_READ_BYTES {
        return (None, Vec::new());
    }
    match std::fs::read_to_string(abs_path) {
        Ok(content) => {
            let refs: Vec<Arc<ReferencedPathData>> =
                references::extract_file_references(&content, extension, rel_path)
                    .into_iter()
                    .map(Arc::new)
                    .collect();
            (Some(content), refs)
        }
        Err(_) => (None, Vec::new()),
    }
}

fn scan_subdirs(skill_dir: &Path, repo_root: &Path, config: &Config) -> Vec<Arc<SubDirData>> {
    let mut result = Vec::new();

    let entries = match std::fs::read_dir(skill_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let rel_dir = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let mut files: Vec<Arc<SubDirFileData>> = Vec::new();
        let mut extensions: BTreeSet<String> = BTreeSet::new();

        if let Ok(dir_entries) = std::fs::read_dir(&path) {
            for file_entry in dir_entries.filter_map(|e| e.ok()) {
                let file_path = file_entry.path();
                if !file_path.is_file() {
                    continue;
                }
                let file_name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let ext = file_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default();
                let rel_file = file_path
                    .strip_prefix(repo_root)
                    .unwrap_or(&file_path)
                    .to_string_lossy()
                    .to_string();

                let is_data = config.is_data_extension(&ext);
                if !ext.is_empty() && !is_data {
                    extensions.insert(ext.clone());
                }

                let (file_content, file_refs) = read_and_extract(&file_path, &ext, &rel_file);

                files.push(Arc::new(SubDirFileData {
                    name: file_name,
                    extension: ext,
                    path: rel_file,
                    is_data_file: is_data,
                    content: file_content,
                    referenced_paths: file_refs,
                }));
            }
        }

        let file_count = files.len() as i64;
        let unique_extensions: Vec<String> = extensions.into_iter().collect();

        result.push(Arc::new(SubDirData {
            name: dir_name,
            path: rel_dir,
            files,
            file_count,
            unique_extensions,
        }));
    }

    result
}

fn scan_root_files(skill_dir: &Path, repo_root: &Path, config: &Config) -> Vec<Arc<SubDirFileData>> {
    let mut result = Vec::new();

    let entries = match std::fs::read_dir(skill_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_name == "SKILL.md" {
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let rel_file = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let is_data = config.is_data_extension(&ext);
        let (file_content, file_refs) = read_and_extract(&path, &ext, &rel_file);

        result.push(Arc::new(SubDirFileData {
            name: file_name,
            extension: ext,
            path: rel_file,
            is_data_file: is_data,
            content: file_content,
            referenced_paths: file_refs,
        }));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_scalar_produces_leaf_entry() {
        let val = serde_yml::Value::String("0.1.0".into());
        let entry = yaml_value_to_metadata("version".into(), &val);
        assert_eq!(entry.key, "version");
        assert_eq!(entry.value, "0.1.0");
        assert!(entry.children.is_empty());
    }

    #[test]
    fn yaml_mapping_produces_children() {
        let yaml: serde_yml::Value = serde_yml::from_str("version: 0.1.0\nauthor: elastic").unwrap();
        let entry = yaml_value_to_metadata("metadata".into(), &yaml);
        assert_eq!(entry.key, "metadata");
        assert_eq!(entry.children.len(), 2);

        let version_child = entry.children.iter().find(|c| c.key == "version").unwrap();
        assert_eq!(version_child.value, "0.1.0");
        assert!(version_child.children.is_empty());

        let author_child = entry.children.iter().find(|c| c.key == "author").unwrap();
        assert_eq!(author_child.value, "elastic");
        assert!(author_child.children.is_empty());
    }

    #[test]
    fn yaml_deeply_nested_mapping() {
        let yaml: serde_yml::Value =
            serde_yml::from_str("outer:\n  inner: deep_value").unwrap();
        let entry = yaml_value_to_metadata("root".into(), &yaml);
        assert_eq!(entry.children.len(), 1);

        let outer = &entry.children[0];
        assert_eq!(outer.key, "outer");
        assert_eq!(outer.children.len(), 1);

        let inner = &outer.children[0];
        assert_eq!(inner.key, "inner");
        assert_eq!(inner.value, "deep_value");
        assert!(inner.children.is_empty());
    }
}
