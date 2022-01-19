use crate::statement::Renderer;
use metamath_knife::outline::OutlineNodeRef;
use metamath_knife::parser::HeadingLevel;
use serde::Serializer;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TocInfo {
    breadcrumb: Vec<ChapterInfo>,
    name: String,
    link: LinkInfo,
    children: Vec<ChapterInfo>,
}

#[derive(Serialize)]
pub(crate) struct ChapterInfo {
    name: String,
    link: LinkInfo,
    stmt_level: bool,
    children: Vec<ChapterInfo>,
}

enum LinkInfo {
    Toc,
    ChapterRef(String),
    StatementRef(String),
}

impl From<&OutlineNodeRef<'_>> for LinkInfo {
    fn from(node: &OutlineNodeRef<'_>) -> Self { 
        match node.get_level() {
            HeadingLevel::Database => LinkInfo::Toc,
            HeadingLevel::Statement => LinkInfo::StatementRef(node.get_name().to_string()),
            _ => LinkInfo::ChapterRef(node.get_ref().to_string())
        }
    }
}

impl Serialize for LinkInfo {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where S: Serializer {
        match self {
            LinkInfo::Toc => serializer.serialize_str("toc"),
            LinkInfo::StatementRef(name) => serializer.serialize_str(&name),
            LinkInfo::ChapterRef(chapter_ref) => serializer.serialize_str(&format!("toc?ref={}", chapter_ref)),
        }
    }
}

pub(crate) fn get_breadcrumb(node: &OutlineNodeRef) -> Vec<ChapterInfo> {
    node.ancestors_iter().map(|node| ChapterInfo {
        name: node.get_name().to_string(),
        stmt_level: node.get_level() == HeadingLevel::Statement,
        link: (&node).into(),
        children: vec![],
    }).collect()
}

impl Renderer {
    pub fn render_toc(&self, _explorer: String, chapter_ref: usize) -> Option<String> {
        let node = if chapter_ref == 0 {
            self.db.root_outline_node()
        } else {
            self.db.get_outline_node_by_ref(chapter_ref)
        };
        let info = TocInfo {
            breadcrumb: get_breadcrumb(&node),
            name: node.get_name().to_string(),
            link: (&node).into(),
            children: node.children_iter().map(|n| ChapterInfo {
                name: n.get_name().to_string(),
                link: (&n).into(),
                stmt_level: n.get_level() == HeadingLevel::Statement,
                children: n.children_iter().map(|c| ChapterInfo {
                    name: c.get_name().to_string(),
                    link: (&c).into(),
                    stmt_level: c.get_level() == HeadingLevel::Statement,
                    children: vec![],
                }).collect()
            }).collect(),
        };
        Some(
            self.templates
                .render("toc", &info)
                .expect("Failed to render"),
        )
    }
}