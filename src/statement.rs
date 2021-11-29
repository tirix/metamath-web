use handlebars::Handlebars;
use metamath_knife::parser::as_str;
use metamath_knife::Database;
use regex::{Captures, Regex};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct StepInfo {
    id: String,
    hyps: Vec<String>,
    label: String,
    expr: String,
}

#[derive(Serialize)]
struct PageInfo {
    label: String,
    comment: String,
    steps: Vec<StepInfo>,
}

trait Replacer: FnMut(&Captures) -> String + Sized + Clone {}

#[derive(Clone)]
pub struct Renderer {
    templates: Arc<Handlebars<'static>>,
    db: Database,
    link_regex: Regex,
}

impl Renderer {
    pub(crate) fn new(db: Database) -> Renderer {
        let mut templates = Handlebars::new();
        templates.register_escape_fn(handlebars::no_escape);
        templates
            .register_template_string("statement", include_str!("statement.hbs"))
            .expect("Unable to parse statement template.");
        let link_regex = Regex::new(r"\~ ([^ ]+) ").unwrap();
        Renderer {
            templates: Arc::new(templates),
            db,
            link_regex,
        }
    }

    pub fn render_statement(&self, label: String) -> Option<String> {
        let sref = self.db.statement(&label)?;

        // Comments
        let comment = if let Some(cmt) = sref.associated_comment() {
            let mut span = cmt.span();
            span.start += 2;
            span.end -= 3;
            let comment = String::from_utf8_lossy(span.as_ref(&cmt.segment().segment.buffer))
                .replace("\n\n", "</p>\n<p>");
            self.link_regex
                .replace_all(comment.as_str(), |caps: &Captures| {
                    format!(
                        "<a href=\"{}\">{}</a>",
                        caps.get(1).map_or("#", |m| m.as_str()),
                        caps.get(1).map_or("-", |m| m.as_str())
                    )
                })
                .to_string()
        } else {
            "(This statement does not have an associated comment)".to_string()
        };

        // Previous and next statements
        // let _prev_label = if let Some(prev_sref) = self.db.prev_sref(sref) { &String::from_utf8_lossy(prev_sref.label()) } else { "" };

        // Proof
        let steps = if let Some(proof_tree) = self.db.get_proof_tree(sref) {
            proof_tree.with_logical_steps(&self.db, |cur, ix, stmt, hyps| StepInfo {
                id: ix.to_string(),
                hyps: hyps.iter().map(usize::to_string).collect::<Vec<String>>(),
                label: as_str(stmt.label()).to_string(),
                expr: "|-".to_string() + &String::from_utf8_lossy(&proof_tree.exprs[cur]),
            })
        } else {
            vec![]
        };

        let info = PageInfo {
            label,
            comment,
            steps,
        };
        Some(
            self.templates
                .render("statement", &info)
                .expect("Failed to render"),
        )
    }
}
