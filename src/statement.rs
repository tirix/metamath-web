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
    bibl_regex: Regex,
    bib_file: String,
}

impl Renderer {
    pub(crate) fn new(db: Database, bib_file: Option<String>) -> Renderer {
        let mut templates = Handlebars::new();
        templates.register_escape_fn(handlebars::no_escape);
        templates
            .register_template_string("statement", include_str!("statement.hbs"))
            .expect("Unable to parse statement template.");
        let link_regex = Regex::new(r"\~ ([^ ]+) ").unwrap();
        let bibl_regex = Regex::new(r"\[([^ ]+)\]").unwrap();
        Renderer {
            templates: Arc::new(templates),
            db,
            link_regex,
            bibl_regex,
            bib_file: bib_file.unwrap_or("".to_string()),
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
            let comment = self.link_regex.replace_all(&comment, |caps: &Captures| {
                format!(
                    "<a href=\"{}\">{}</a>",
                    caps.get(1).map_or("#", |m| m.as_str()),
                    caps.get(1).map_or("-", |m| m.as_str())
                )
            });
            let comment = self.bibl_regex.replace_all(&comment, |caps: &Captures| {
                format!(
                    "<a href=\"{}#{}\">{}</a>",
                    self.bib_file,
                    caps.get(1).map_or("", |m| m.as_str()),
                    caps.get(1).map_or("", |m| m.as_str())
                )
            });
            comment.to_string()
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
