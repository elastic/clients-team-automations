pub mod vertex;

use std::sync::Arc;

use trustfall::FieldValue;
use trustfall::provider::{
    BasicAdapter, ContextIterator, ContextOutcomeIterator, EdgeParameters,
    VertexIterator, resolve_neighbors_with, resolve_property_with,
};

use crate::data::SkillsData;
use vertex::Vertex;

#[derive(Debug, Clone)]
pub struct SkillsAdapter {
    data: Arc<SkillsData>,
}

impl SkillsAdapter {
    pub fn new(data: SkillsData) -> Self {
        Self {
            data: Arc::new(data),
        }
    }
}

impl<'a> BasicAdapter<'a> for SkillsAdapter {
    type Vertex = Vertex;

    fn resolve_starting_vertices(
        &self,
        edge_name: &str,
        _parameters: &EdgeParameters,
    ) -> VertexIterator<'a, Self::Vertex> {
        match edge_name {
            "Skill" => {
                let skills: Vec<_> = self
                    .data
                    .skills
                    .iter()
                    .map(|s| Vertex::Skill(s.clone()))
                    .collect();
                Box::new(skills.into_iter())
            }
            "GroupFolder" => {
                let folders: Vec<_> = self
                    .data
                    .group_folders
                    .iter()
                    .map(|g| Vertex::GroupFolder(g.clone()))
                    .collect();
                Box::new(folders.into_iter())
            }
            "DiscoveredSkillFile" => {
                let files: Vec<_> = self
                    .data
                    .discovered_files
                    .iter()
                    .map(|d| Vertex::DiscoveredSkillFile(d.clone()))
                    .collect();
                Box::new(files.into_iter())
            }
            "GitHubOrg" => {
                let org = self.data.github_org.clone();
                Box::new(std::iter::once(Vertex::GitHubOrg(org)))
            }
            _ => unreachable!("unknown starting edge: {edge_name}"),
        }
    }

    fn resolve_property<V: trustfall::provider::AsVertex<Self::Vertex> + 'a>(
        &self,
        contexts: ContextIterator<'a, V>,
        type_name: &str,
        property_name: &str,
    ) -> ContextOutcomeIterator<'a, V, FieldValue> {
        match (type_name, property_name) {
            // --- Skill properties ---
            ("Skill", "folder_name") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().folder_name.clone().into()
            }),
            ("Skill", "group_folder") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().group_folder.clone().into()
            }),
            ("Skill", "path") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().path.clone().into()
            }),
            ("Skill", "skill_file_path") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().skill_file_path.clone().into()
            }),
            ("Skill", "depth") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().depth.into()
            }),
            ("Skill", "has_frontmatter") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().has_frontmatter.into()
            }),
            ("Skill", "raw_frontmatter") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.raw_frontmatter
                    .as_ref()
                    .map(|r| FieldValue::from(r.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("Skill", "name") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.name
                    .as_ref()
                    .map(|n| FieldValue::from(n.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("Skill", "description") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.description
                    .as_ref()
                    .map(|d| FieldValue::from(d.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("Skill", "license") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.license
                    .as_ref()
                    .map(|l| FieldValue::from(l.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("Skill", "compatibility") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.compatibility
                    .as_ref()
                    .map(|c| FieldValue::from(c.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("Skill", "allowed_tools") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.allowed_tools
                    .as_ref()
                    .map(|a| FieldValue::from(a.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("Skill", "description_length") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().description_length.into()
            }),
            ("Skill", "description_word_count") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().description_word_count.into()
            }),
            ("Skill", "total_line_count") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().total_line_count.into()
            }),
            ("Skill", "body_line_count") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().body_line_count.into()
            }),
            ("Skill", "has_title_heading") => resolve_property_with(contexts, |v| {
                v.as_skill().unwrap().has_title_heading.into()
            }),
            ("Skill", "title_heading") => resolve_property_with(contexts, |v| {
                let s = v.as_skill().unwrap();
                s.title_heading
                    .as_ref()
                    .map(|t| FieldValue::from(t.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),

            // --- GroupFolder properties ---
            ("GroupFolder", "name") => resolve_property_with(contexts, |v| {
                v.as_group_folder().unwrap().name.clone().into()
            }),
            ("GroupFolder", "path") => resolve_property_with(contexts, |v| {
                v.as_group_folder().unwrap().path.clone().into()
            }),
            ("GroupFolder", "skill_count") => resolve_property_with(contexts, |v| {
                v.as_group_folder().unwrap().skill_count.into()
            }),

            // --- DiscoveredSkillFile properties ---
            ("DiscoveredSkillFile", "path") => resolve_property_with(contexts, |v| {
                v.as_discovered_skill_file().unwrap().path.clone().into()
            }),
            ("DiscoveredSkillFile", "parent_dir") => resolve_property_with(contexts, |v| {
                v.as_discovered_skill_file()
                    .unwrap()
                    .parent_dir
                    .clone()
                    .into()
            }),
            ("DiscoveredSkillFile", "depth") => resolve_property_with(contexts, |v| {
                v.as_discovered_skill_file().unwrap().depth.into()
            }),
            // --- Section properties ---
            ("Section", "level") => resolve_property_with(contexts, |v| {
                v.as_section().unwrap().level.into()
            }),
            ("Section", "heading") => resolve_property_with(contexts, |v| {
                v.as_section().unwrap().heading.clone().into()
            }),
            ("Section", "line_number") => resolve_property_with(contexts, |v| {
                v.as_section().unwrap().line_number.into()
            }),
            ("Section", "content") => resolve_property_with(contexts, |v| {
                v.as_section().unwrap().content.clone().into()
            }),
            ("Section", "content_line_count") => resolve_property_with(contexts, |v| {
                v.as_section().unwrap().content_line_count.into()
            }),

            // --- CodeBlock properties ---
            ("CodeBlock", "language") => resolve_property_with(contexts, |v| {
                let cb = v.as_code_block().unwrap();
                cb.language
                    .as_ref()
                    .map(|l| FieldValue::from(l.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("CodeBlock", "has_language_tag") => resolve_property_with(contexts, |v| {
                v.as_code_block().unwrap().has_language_tag.into()
            }),
            ("CodeBlock", "line_number") => resolve_property_with(contexts, |v| {
                v.as_code_block().unwrap().line_number.into()
            }),
            ("CodeBlock", "content") => resolve_property_with(contexts, |v| {
                v.as_code_block().unwrap().content.clone().into()
            }),

            // --- SubDir properties ---
            ("SubDir", "name") => resolve_property_with(contexts, |v| {
                v.as_sub_dir().unwrap().name.clone().into()
            }),
            ("SubDir", "path") => resolve_property_with(contexts, |v| {
                v.as_sub_dir().unwrap().path.clone().into()
            }),
            ("SubDir", "file_count") => resolve_property_with(contexts, |v| {
                v.as_sub_dir().unwrap().file_count.into()
            }),
            ("SubDir", "unique_extensions") => resolve_property_with(contexts, |v| {
                let sd = v.as_sub_dir().unwrap();
                let list: Vec<FieldValue> = sd
                    .unique_extensions
                    .iter()
                    .map(|e| FieldValue::from(e.as_str()))
                    .collect();
                FieldValue::List(list.into())
            }),
            ("SubDir", "unique_extension_count") => resolve_property_with(contexts, |v| {
                let sd = v.as_sub_dir().unwrap();
                FieldValue::Int64(sd.unique_extensions.len() as i64)
            }),

            // --- SubDirFile properties ---
            ("SubDirFile", "name") => resolve_property_with(contexts, |v| {
                v.as_sub_dir_file().unwrap().name.clone().into()
            }),
            ("SubDirFile", "extension") => resolve_property_with(contexts, |v| {
                v.as_sub_dir_file().unwrap().extension.clone().into()
            }),
            ("SubDirFile", "path") => resolve_property_with(contexts, |v| {
                v.as_sub_dir_file().unwrap().path.clone().into()
            }),
            ("SubDirFile", "is_data_file") => resolve_property_with(contexts, |v| {
                v.as_sub_dir_file().unwrap().is_data_file.into()
            }),
            ("SubDirFile", "content") => resolve_property_with(contexts, |v| {
                let f = v.as_sub_dir_file().unwrap();
                f.content
                    .as_ref()
                    .map(|c| FieldValue::from(c.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),

            // --- MetadataEntry properties ---
            ("MetadataEntry", "key") => resolve_property_with(contexts, |v| {
                v.as_metadata_entry().unwrap().key.clone().into()
            }),
            ("MetadataEntry", "value") => resolve_property_with(contexts, |v| {
                v.as_metadata_entry().unwrap().value.clone().into()
            }),

            // --- Span properties ---
            ("Span", "filename") => resolve_property_with(contexts, |v| {
                v.as_span().unwrap().filename.clone().into()
            }),
            ("Span", "begin_line") => resolve_property_with(contexts, |v| {
                v.as_span().unwrap().begin_line.into()
            }),
            ("Span", "end_line") => resolve_property_with(contexts, |v| {
                v.as_span().unwrap().end_line.into()
            }),

            // --- GitHubOrg properties ---
            ("GitHubOrg", "name") => resolve_property_with(contexts, |v| {
                v.as_git_hub_org().unwrap().name.clone().into()
            }),
            ("GitHubOrg", "teams_loaded") => resolve_property_with(contexts, |v| {
                v.as_git_hub_org().unwrap().teams_loaded.into()
            }),
            ("GitHubOrg", "team_count") => resolve_property_with(contexts, |v| {
                FieldValue::Int64(v.as_git_hub_org().unwrap().teams.len() as i64)
            }),

            // --- GitHubTeam properties ---
            ("GitHubTeam", "slug") => resolve_property_with(contexts, |v| {
                v.as_git_hub_team().unwrap().slug.clone().into()
            }),
            ("GitHubTeam", "name") => resolve_property_with(contexts, |v| {
                v.as_git_hub_team().unwrap().name.clone().into()
            }),
            ("GitHubTeam", "description") => resolve_property_with(contexts, |v| {
                let t = v.as_git_hub_team().unwrap();
                t.description
                    .as_ref()
                    .map(|d| FieldValue::from(d.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),

            // --- ReferencedPath properties ---
            ("ReferencedPath", "raw_path") => resolve_property_with(contexts, |v| {
                v.as_referenced_path().unwrap().raw_path.clone().into()
            }),
            ("ReferencedPath", "resolved_path") => resolve_property_with(contexts, |v| {
                let rp = v.as_referenced_path().unwrap();
                rp.resolved_path
                    .as_ref()
                    .map(|p| FieldValue::from(p.as_str()))
                    .unwrap_or(FieldValue::Null)
            }),
            ("ReferencedPath", "kind") => resolve_property_with(contexts, |v| {
                v.as_referenced_path().unwrap().kind.clone().into()
            }),
            ("ReferencedPath", "line_number") => resolve_property_with(contexts, |v| {
                v.as_referenced_path().unwrap().line_number.into()
            }),

            _ => unreachable!("unknown property: {type_name}.{property_name}"),
        }
    }

    fn resolve_neighbors<V: trustfall::provider::AsVertex<Self::Vertex> + 'a>(
        &self,
        contexts: ContextIterator<'a, V>,
        type_name: &str,
        edge_name: &str,
        _parameters: &EdgeParameters,
    ) -> ContextOutcomeIterator<'a, V, VertexIterator<'a, Self::Vertex>> {
        let data = self.data.clone();
        match (type_name, edge_name) {
            ("Skill", "metadata") => resolve_neighbors_with(contexts, move |v| {
                let skill = v.as_skill().unwrap();
                let items: Vec<Vertex> = skill
                    .metadata
                    .iter()
                    .map(|m| Vertex::MetadataEntry(m.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),
            ("MetadataEntry", "children") => resolve_neighbors_with(contexts, move |v| {
                let entry = v.as_metadata_entry().unwrap();
                let items: Vec<Vertex> = entry
                    .children
                    .iter()
                    .map(|c| Vertex::MetadataEntry(c.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),
            ("Skill", "section") => resolve_neighbors_with(contexts, move |v| {
                let skill = v.as_skill().unwrap();
                let items: Vec<Vertex> = skill
                    .sections
                    .iter()
                    .map(|s| Vertex::Section(s.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),
            ("Skill", "sub_dir") => resolve_neighbors_with(contexts, move |v| {
                let skill = v.as_skill().unwrap();
                let items: Vec<Vertex> = skill
                    .sub_dirs
                    .iter()
                    .map(|s| Vertex::SubDir(s.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),
            ("Skill", "referenced_path") => resolve_neighbors_with(contexts, move |v| {
                let skill = v.as_skill().unwrap();
                let items: Vec<Vertex> = skill
                    .referenced_paths
                    .iter()
                    .map(|rp| Vertex::ReferencedPath(rp.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),
            ("Skill", "all_other_skills") => {
                let all_skills = data.skills.clone();
                resolve_neighbors_with(contexts, move |v| {
                    let skill = v.as_skill().unwrap();
                    let current_path = &skill.skill_file_path;
                    let items: Vec<Vertex> = all_skills
                        .iter()
                        .filter(|s| &s.skill_file_path != current_path)
                        .map(|s| Vertex::Skill(s.clone()))
                        .collect();
                    Box::new(items.into_iter())
                })
            }
            ("Skill", "span") => resolve_neighbors_with(contexts, move |v| {
                let skill = v.as_skill().unwrap();
                let item = Vertex::Span(skill.span.clone());
                Box::new(std::iter::once(item))
            }),
            ("Skill", "frontmatter_span") => resolve_neighbors_with(contexts, move |v| {
                let skill = v.as_skill().unwrap();
                match &skill.frontmatter_span {
                    Some(span) => Box::new(std::iter::once(Vertex::Span(span.clone()))),
                    None => Box::new(std::iter::empty()),
                }
            }),

            ("GroupFolder", "skill") => {
                let all_skills = data.skills.clone();
                resolve_neighbors_with(contexts, move |v| {
                    let gf = v.as_group_folder().unwrap();
                    let items: Vec<Vertex> = gf
                        .skill_indices
                        .iter()
                        .filter_map(|&idx| all_skills.get(idx))
                        .map(|s| Vertex::Skill(s.clone()))
                        .collect();
                    Box::new(items.into_iter())
                })
            }

            ("DiscoveredSkillFile", "skill") => {
                let all_skills = data.skills.clone();
                resolve_neighbors_with(contexts, move |v| {
                    let dsf = v.as_discovered_skill_file().unwrap();
                    match dsf.skill_index {
                        Some(idx) => match all_skills.get(idx) {
                            Some(s) => Box::new(std::iter::once(Vertex::Skill(s.clone()))),
                            None => Box::new(std::iter::empty()),
                        },
                        None => Box::new(std::iter::empty()),
                    }
                })
            }
            ("DiscoveredSkillFile", "span") => resolve_neighbors_with(contexts, move |v| {
                let dsf = v.as_discovered_skill_file().unwrap();
                Box::new(std::iter::once(Vertex::Span(dsf.span.clone())))
            }),

            ("Section", "code_block") => resolve_neighbors_with(contexts, move |v| {
                let section = v.as_section().unwrap();
                let items: Vec<Vertex> = section
                    .code_blocks
                    .iter()
                    .map(|cb| Vertex::CodeBlock(Arc::new(cb.clone())))
                    .collect();
                Box::new(items.into_iter())
            }),

            ("SubDir", "file") => resolve_neighbors_with(contexts, move |v| {
                let sd = v.as_sub_dir().unwrap();
                let items: Vec<Vertex> = sd
                    .files
                    .iter()
                    .map(|f| Vertex::SubDirFile(f.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),

            ("SubDirFile", "referenced_path") => resolve_neighbors_with(contexts, move |v| {
                let f = v.as_sub_dir_file().unwrap();
                let items: Vec<Vertex> = f
                    .referenced_paths
                    .iter()
                    .map(|rp| Vertex::ReferencedPath(rp.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),

            ("Skill", "github_org") => {
                let github_org = data.github_org.clone();
                resolve_neighbors_with(contexts, move |_v| {
                    Box::new(std::iter::once(Vertex::GitHubOrg(github_org.clone())))
                })
            }

            ("GitHubOrg", "team") => resolve_neighbors_with(contexts, move |v| {
                let org = v.as_git_hub_org().unwrap();
                let items: Vec<Vertex> = org
                    .teams
                    .iter()
                    .map(|t| Vertex::GitHubTeam(t.clone()))
                    .collect();
                Box::new(items.into_iter())
            }),

            _ => unreachable!("unknown edge: {type_name}.{edge_name}"),
        }
    }

    fn resolve_coercion<V: trustfall::provider::AsVertex<Self::Vertex> + 'a>(
        &self,
        _contexts: ContextIterator<'a, V>,
        _type_name: &str,
        _coerce_to_type: &str,
    ) -> ContextOutcomeIterator<'a, V, bool> {
        unreachable!("no coercions in the skills schema")
    }
}
