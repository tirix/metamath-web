use handlebars::Handlebars;
use metamath_knife::parser::as_str;
use metamath_knife::parser::StatementType;
use metamath_knife::parser::StatementRef;
use metamath_knife::Database;
use metamath_knife::Formula;
use metamath_knife::proof::ProofTreeArray;
use regex::{Captures, Regex};
use serde::Serialize;
use std::sync::Arc;
#[cfg(feature = "sts")]
use crate::sts::StsDefinition;
use crate::toc::ChapterInfo;
use crate::toc::get_breadcrumb;

#[derive(Serialize)]
struct HypInfo {
    label: String,
    expr: String,
}

#[derive(Serialize)]
struct StepInfo {
    id: String,
    hyps: Vec<String>,
    label: String,
    expr: String,
}

#[derive(Serialize)]
struct PageInfo {
    header: String,
    label: String,
    statement_type: String,
    comment: String,
    expr: String,
    breadcrumb: Vec<ChapterInfo>,
    hyps: Vec<HypInfo>,
    is_proof: bool,
    steps: Vec<StepInfo>,
}

trait Replacer: FnMut(&Captures) -> String + Sized + Clone {}

#[derive(Clone)]
pub struct Renderer {
    pub(crate) templates: Arc<Handlebars<'static>>,
    pub(crate) db: Database,
    contrib_regex: Regex,
    discouraged_regex: Regex,
    link_regex: Regex,
    bibl_regex: Regex,
    bib_file: String,
    #[cfg(feature = "sts")]
    sts: StsDefinition,
}

#[derive(Clone)]
enum ExpressionRenderer {
    ASCII,
//    HTML,     // To be completed
//    Unicode,  // To be completed
#[cfg(feature = "sts")]
    STS(StsDefinition),
}

impl ExpressionRenderer {
    fn render_statement(&self, sref: &StatementRef, database: &Database, use_provables: bool) -> Result<String, String> {
        match self {
            ExpressionRenderer::ASCII => self.render_formula(database.stmt_parse_result().get_formula(sref).ok_or("Formula not found")?, database, use_provables),
            #[cfg(feature = "sts")]
            ExpressionRenderer::STS(sts) => sts.render_statement(sref, use_provables),
        }
    }

    fn render_formula(&self, formula: &Formula, database: &Database, use_provables: bool) -> Result<String, String> {
        match self {
            ExpressionRenderer::ASCII => Ok(format!("<pre>{}</pre>", formula.as_ref(database)).replace("wff ", " |- ")),
            #[cfg(feature = "sts")]
            ExpressionRenderer::STS(sts) => sts.render_formula(formula, use_provables),
        }
    }

    fn render_expression(self, proof_tree: &ProofTreeArray, tree_index: usize, use_provables: bool) -> Result<String, String> {
        match self {
            ExpressionRenderer::ASCII => Ok(format!("<pre> |- {}</pre>", &String::from_utf8_lossy(&proof_tree.exprs[tree_index]))),
            #[cfg(feature = "sts")]
            ExpressionRenderer::STS(sts) => sts.render_expression(proof_tree, tree_index, use_provables),
        }
    }

    fn get_header(&self) -> String {
        match self {
            ExpressionRenderer::ASCII => "".to_string(),
            #[cfg(feature = "sts")]
            ExpressionRenderer::STS(sts) => sts.header.clone(),
        }
    }
}

impl Renderer {
    pub(crate) fn new(db: Database, bib_file: Option<String>,
        #[cfg(feature = "sts")]
        sts: StsDefinition
    ) -> Renderer {
        let mut templates = Handlebars::new();
        templates.register_escape_fn(handlebars::no_escape);
        templates
            .register_template_string("statement", include_str!("statement.hbs"))
            .expect("Unable to parse statement template.");
        templates
            .register_template_string("toc", include_str!("toc.hbs"))
            .expect("Unable to parse table of contents template.");
        let contrib_regex = Regex::new(r"\((Contributed|Revised|Proof[ \n]+shortened)[ \n]+by[ \n]+(?s)(.+?),[ \n]+(\d{1,2}-\w\w\w-\d{4})\.\)").unwrap();
        let discouraged_regex = Regex::new(r"\(New usage is discouraged\.\)|\(Proof modification is discouraged\.\)").unwrap();
        let link_regex = Regex::new(r"\~ ([^ \n]+)[ \n]+").unwrap();
        let bibl_regex = Regex::new(r"\[([^ \n]+)\]").unwrap();
        Renderer {
            templates: Arc::new(templates),
            db,
            contrib_regex,
            discouraged_regex,
            link_regex,
            bibl_regex,
            bib_file: bib_file.unwrap_or("".to_string()),
            #[cfg(feature = "sts")]
            sts,
        }
    }

    fn get_expression_renderer(&self, explorer: String) -> Option<ExpressionRenderer> {
        match explorer.as_str() {
            "mpeascii" => Some(ExpressionRenderer::ASCII),
            #[cfg(feature = "sts")]
            "mpests" => Some(ExpressionRenderer::STS(self.sts.clone())),
            _ => None,
        }
    }

    pub fn render_statement(&self, explorer: String, label: String) -> Option<String> {
        let sref = self.db.statement(&label)?;
        let expression_renderer = self.get_expression_renderer(explorer)?;

        // Header
        let header = expression_renderer.get_header();

        // Table of Contents - Breadcrumb
        let breadcrumb = get_breadcrumb(&self.db.get_outline_node(sref));

        // Comments
        let comment = if let Some(cmt) = sref.associated_comment() {
            let mut span = cmt.span();
            span.start += 2;
            span.end -= 3;
            let comment = String::from_utf8_lossy(span.as_ref(&cmt.segment().segment.buffer))
                .replace("\n\n", "</p>\n<p>");
            let comment = self.contrib_regex.replace_all(&comment, |caps: &Captures| {
                format!(
                    "<span class=\"contrib\">({} by <a href=\"/contributors#{}\">{}</a>, {})</span>",
                    caps.get(1).expect("Contribution Regex did not return a contribution type").as_str(),
                    caps.get(2).expect("Contribution Regex did not return a contributor").as_str(),
                    caps.get(2).expect("Contribution Regex did not return a contributor").as_str(),
                    caps.get(3).expect("Contribution Regex did not return a contribution date").as_str(),
                )
            });
            let comment = self.discouraged_regex.replace_all(&comment, |caps: &Captures| {
                format!(
                    "<span class=\"discouraged\">{}</span>",
                    caps.get(0).unwrap().as_str(),
                )
            });
            let comment = self.link_regex.replace_all(&comment, |caps: &Captures| {
                format!(
                    "<a href=\"{}\" class=\"label\">{}</a> ",
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
//            Double tildes ~~ shall be substituted with single tildes, see link in ~ dn1

//            Anything inside <HTML> shall be unchanged
//            _..._ -> to italics <em></em>, except if part of external hyperlinks
//            See mmwtex.c
            comment.to_string()
        } else {
            "(This statement does not have an associated comment)".to_string()
        };

        // Previous and next statements
        // let _prev_label = if let Some(prev_sref) = self.db.prev_sref(sref) { &String::from_utf8_lossy(prev_sref.label()) } else { "" };

        // Proof or Syntax proof
        let (is_proof, steps) = match sref.statement_type() {
            StatementType::Provable =>
                (true, match self.db.get_proof_tree(sref) {
                    Some(proof_tree) => proof_tree.with_logical_steps(&self.db, |cur, ix, stmt, hyps| StepInfo {
                        id: ix.to_string(),
                        hyps: hyps.iter().map(usize::to_string).collect::<Vec<String>>(),
                        label: as_str(stmt.label()).to_string(),
                        expr: expression_renderer.clone().render_expression(&proof_tree, cur, true)
                            .unwrap_or_else(|e| format!("Could not format {} : {}", 
                            &String::from_utf8_lossy(&proof_tree.exprs[cur]), e)),
                        }),
                    None => vec![],
                }),
            StatementType::Axiom|StatementType::Essential|StatementType::Floating =>
                (false, match self.db.stmt_parse_result().get_formula(&sref) {
                    Some(formula) => {
                        let proof_tree = self.db.get_syntax_proof_tree(formula);
                        proof_tree.with_steps(&self.db, |cur, stmt, hyps| StepInfo {
                            id: cur.to_string(),
                            hyps: hyps.iter().map(usize::to_string).collect::<Vec<String>>(),
                            label: as_str(stmt.label()).to_string(),
                            expr: expression_renderer.clone().render_expression(&proof_tree, cur, false)
                                .unwrap_or_else(|e| format!("Could not format {} : {}", 
                                &String::from_utf8_lossy(&proof_tree.exprs[cur]), e)),
                            })
                    },
                    None => vec![],
                }),
            _ => (false, vec![]),
        };

        // Statement type
        let statement_type = if is_proof { "Theorem".to_string() }
            else if steps.len()==0 { "Syntax definition".to_string() } 
            else if label.starts_with("df-") { "Definition".to_string() }
            else { "Axiom".to_string() };

        // Statement assertion
        let expr = expression_renderer.render_statement(&sref, &self.db, is_proof).unwrap_or_else(|e| format!("Could not format assertion : {}", e));

        // Hypotheses
        let hyps = self.db.scope_result().get(sref.label())?.as_ref(&self.db).essentials().map(|(label, formula)| {
            HypInfo {
                label: as_str(self.db.name_result().atom_name(label)).to_string(),
                expr: expression_renderer.render_formula(formula, &self.db, is_proof).unwrap_or_else(|e| e),
            }
        }).collect();

        let info = PageInfo {
            header,
            breadcrumb,
            label,
            statement_type,
            comment,
            expr,
            hyps,
            is_proof,
            steps,
        };
        Some(
            self.templates
                .render("statement", &info)
                .expect("Failed to render"),
        )
    }
}
