use handlebars::Handlebars;
use metamath_knife::parser::as_str;
use metamath_knife::Database;
use regex::Regex;
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

#[derive(Clone)]
pub struct Renderer {
    templates: Arc<Handlebars<'static>>,
    db: Database,
}

impl Renderer {
    pub(crate) fn new(db: Database) -> Renderer {
        let mut templates = Handlebars::new();
        templates
            .register_template_string("statement", include_str!("statement.hbs"))
            .expect("Unable to parse statement template.");
        Renderer {
            templates: Arc::new(templates),
            db,
        }
    }

    pub fn render_statement(&self, label: String) -> Option<String> {
        let sref = self.db.statement(&label)?;

        // Comments
        let comment = if let Some(cmt) = sref.associated_comment() {
            let mut span = cmt.span();
            span.start += 2;
            span.end -= 3;
            Regex::new(r"\n +").unwrap().replace(
                &String::from_utf8_lossy(span.as_ref(&cmt.segment().segment.buffer)),
                "\n  ",
            )
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
