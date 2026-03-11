use std::sync::Arc;

use trustfall_derive::TrustfallEnumVertex;

use crate::data::{
    DiscoveredDirectoryData, DiscoveredSkillFileData, GitHubOrgData, GitHubTeamData,
    GroupFolderData, MetadataEntryData, SkillData, SpanData, SubDirData, SubDirFileData,
};
use crate::markdown::{CodeBlockData, SectionData};
use crate::references::ReferencedPathData;

#[derive(Debug, Clone, TrustfallEnumVertex)]
pub enum Vertex {
    Skill(Arc<SkillData>),
    GroupFolder(Arc<GroupFolderData>),
    DiscoveredSkillFile(Arc<DiscoveredSkillFileData>),
    DiscoveredDirectory(Arc<DiscoveredDirectoryData>),
    Section(Arc<SectionData>),
    CodeBlock(Arc<CodeBlockData>),
    SubDir(Arc<SubDirData>),
    SubDirFile(Arc<SubDirFileData>),
    MetadataEntry(Arc<MetadataEntryData>),
    Span(Arc<SpanData>),
    GitHubOrg(Arc<GitHubOrgData>),
    GitHubTeam(Arc<GitHubTeamData>),
    ReferencedPath(Arc<ReferencedPathData>),
}
