use std::sync::Arc;

use trustfall_derive::TrustfallEnumVertex;

use crate::data::{
    DiscoveredSkillFileData, GroupFolderData, MetadataEntryData, SkillData, SpanData, SubDirData,
    SubDirFileData,
};
use crate::markdown::{CodeBlockData, SectionData};

#[derive(Debug, Clone, TrustfallEnumVertex)]
pub enum Vertex {
    Skill(Arc<SkillData>),
    GroupFolder(Arc<GroupFolderData>),
    DiscoveredSkillFile(Arc<DiscoveredSkillFileData>),
    Section(Arc<SectionData>),
    CodeBlock(Arc<CodeBlockData>),
    SubDir(Arc<SubDirData>),
    SubDirFile(Arc<SubDirFileData>),
    MetadataEntry(Arc<MetadataEntryData>),
    Span(Arc<SpanData>),
}
