use crate::statement::Renderer;
use crate::statement::TypesettingInfo;
use metamath_knife::outline::OutlineNodeRef;
use metamath_knife::parser::HeadingLevel;
use serde::Serialize;
use serde::Serializer;

#[derive(Serialize)]
pub(crate) struct TocInfo {
    nav: NavInfo,
    name: String,
    comment: Option<String>,
    explorer: String,
    link: LinkInfo,
    children: Vec<ChapterInfo>,
}

#[derive(Serialize)]
pub(crate) struct ChapterInfo {
    name: String,
    link: LinkInfo,
    stmt_level: bool,
    children: Vec<ChapterInfo>,
    index: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct NavInfo {
    breadcrumb: Vec<ChapterInfo>,
    next: Option<ChapterInfo>,
    typesettings: Vec<TypesettingInfo>,
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
            _ => LinkInfo::ChapterRef(node.get_ref().to_string()),
        }
    }
}

impl From<&(Option<usize>, OutlineNodeRef<'_>)> for ChapterInfo {
    fn from(data: &(Option<usize>, OutlineNodeRef<'_>)) -> Self {
        let &(index, ref node) = data;
        ChapterInfo {
            name: node.get_name().to_string(),
            index,
            stmt_level: node.get_level() == HeadingLevel::Statement,
            link: node.into(),
            children: vec![],
        }
    }
}

impl Serialize for LinkInfo {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        match self {
            LinkInfo::Toc => serializer.serialize_str("toc"),
            LinkInfo::StatementRef(name) => serializer.serialize_str(name),
            LinkInfo::ChapterRef(chapter_ref) => {
                serializer.serialize_str(&format!("toc?ref={}", chapter_ref))
            }
        }
    }
}

impl Renderer {
    pub(crate) fn get_nav(&self, node: &OutlineNodeRef) -> NavInfo {
        NavInfo {
            breadcrumb: self.get_breadcrumb(node),
            next: node.next().map(|n| (&(None, n)).into()),
            typesettings: Renderer::get_typesettings(),
        }
    }

    pub(crate) fn get_breadcrumb(&self, node: &OutlineNodeRef) -> Vec<ChapterInfo> {
        let mut breadcrumb: Vec<ChapterInfo> = node
            .ancestors_iter()
            .map(|node| (&(Renderer::get_index(&node), node)).into())
            .collect();
        breadcrumb.reverse();
        breadcrumb
    }

    fn get_index(node: &OutlineNodeRef) -> Option<usize> {
        node.parent().and_then(|parent| {
            parent.children_iter().enumerate().find_map(|(i, n)| {
                if n.get_statement().address() == node.get_statement().address() {
                    Some(i + 1)
                } else {
                    None
                }
            })
        })
    }

    fn get_comment(&self, node: &OutlineNodeRef) -> Option<String> {
        let stmt = node.get_statement();
        Some(self.render_comment_new(
            &stmt.segment().segment.buffer,
            stmt.as_heading_comment()?.content,
        ))
    }

    pub fn render_toc(&self, explorer: String, chapter_ref: usize) -> Option<String> {
        let node = if chapter_ref == 0 {
            self.db.root_outline_node()
        } else {
            self.db.get_outline_node_by_ref(chapter_ref)
        };
        let comment = self.get_comment(&node);
        let info = TocInfo {
            nav: self.get_nav(&node),
            explorer,
            name: node.get_name().to_string(),
            comment,
            link: (&node).into(),
            children: node
                .children_iter()
                .map(|n| ChapterInfo {
                    name: n.get_name().to_string(),
                    index: None,
                    link: (&n).into(),
                    stmt_level: n.get_level() == HeadingLevel::Statement,
                    children: n.children_iter().map(|c| (&(None, c)).into()).collect(),
                })
                .collect(),
        };
        Some(
            self.templates
                .render("toc", &info)
                .expect("Failed to render"),
        )
    }
}
