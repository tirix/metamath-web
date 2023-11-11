#[cfg(feature = "sts")]
use crate::sts::StsDefinition;
use crate::toc::NavInfo;
use crate::uni::UnicodeRenderer;
use handlebars::Handlebars;
use metamath_knife::comment_parser::CommentItem;
use metamath_knife::comment_parser::CommentParser;
use metamath_knife::grammar::FormulaToken;
use metamath_knife::proof::ProofTreeArray;
use metamath_knife::statement::as_str;
use metamath_knife::statement::StatementRef;
use metamath_knife::statement::StatementType;
use metamath_knife::Database;
use metamath_knife::Formula;
use metamath_knife::Span;
use regex::{Captures, Regex};
use serde::Serialize;
use std::sync::Arc;

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
    r#type: String,
    link: bool,
}

#[derive(Serialize)]
struct PageInfo {
    header: String,
    explorer: String,
    label: String,
    statement_type: String,
    comment: String,
    expr: String,
    nav: NavInfo,
    hyps: Vec<HypInfo>,
    is_proof: bool,
    steps: Vec<StepInfo>,
}

#[derive(Serialize)]
pub(crate) struct TypesettingInfo {
    dir: &'static str,
    name: &'static str,
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
    math_regex: Regex,
    underline_regex: Regex,
    #[cfg(feature = "sts")]
    sts: StsDefinition,
    uni: UnicodeRenderer,
}

#[derive(Clone)]
enum ExpressionRenderer {
    Ascii,
    Unicode(UnicodeRenderer),
    #[cfg(feature = "sts")]
    Sts(StsDefinition),
}

impl ExpressionRenderer {
    fn render_statement(
        &self,
        sref: &StatementRef,
        database: &Database,
        use_provables: bool,
    ) -> Result<String, String> {
        match self {
            ExpressionRenderer::Ascii => self.render_formula(
                &self.get_formula(sref, database, use_provables)?,
                database,
                use_provables,
            ),
            ExpressionRenderer::Unicode(uni) => uni.render_statement(sref),
            #[cfg(feature = "sts")]
            ExpressionRenderer::Sts(sts) => sts.render_formula(
                &self.get_formula(sref, database, use_provables)?,
                use_provables,
            ),
        }
    }

    #[inline]
    fn get_formula(
        &self,
        sref: &StatementRef,
        database: &Database,
        use_provables: bool,
    ) -> Result<Formula, String> {
        if use_provables {
            database
                .stmt_parse_result()
                .get_formula(sref)
                .ok_or("Unknown statement".into())
                .cloned()
        } else {
            let nset = database.name_result();
            let grammar = database.grammar_result();
            let mut tokens = sref.math_iter();
            let _typecode = nset.get_atom(&tokens.next().unwrap());
            grammar
                .parse_formula(
                    &mut tokens.map(|t| {
                        Ok(FormulaToken {
                            symbol: nset.get_atom(&t),
                            span: metamath_knife::Span::NULL,
                        })
                    }),
                    &grammar.typecodes(),
                    true,
                    nset,
                )
                .map_err(|e| format!("Could not parse formula (GF): {}", e))
        }
    }

    fn render_formula(
        &self,
        formula: &Formula,
        database: &Database,
        use_provables: bool,
    ) -> Result<String, String> {
        match self {
            ExpressionRenderer::Ascii => {
                let s = format!("<pre>{}</pre>", formula.as_ref(database));
                Ok(if use_provables {
                    s.replace("wff ", " |- ")
                } else {
                    s
                })
            }
            ExpressionRenderer::Unicode(uni) => uni.render_formula(formula),
            #[cfg(feature = "sts")]
            ExpressionRenderer::Sts(sts) => sts.render_formula(formula, use_provables),
        }
    }

    fn render_expression(
        self,
        proof_tree: &ProofTreeArray,
        tree_index: usize,
        use_provables: bool,
    ) -> Result<String, String> {
        match self {
            ExpressionRenderer::Ascii => Ok(format!(
                "<pre> |- {}</pre>",
                &String::from_utf8_lossy(&proof_tree.exprs().unwrap()[tree_index])
            )),
            ExpressionRenderer::Unicode(uni) => {
                uni.render_formula(&ExpressionRenderer::as_formula(
                    &uni.database,
                    proof_tree,
                    tree_index,
                    use_provables,
                )?)
            }
            #[cfg(feature = "sts")]
            ExpressionRenderer::Sts(sts) => sts.render_formula(
                &ExpressionRenderer::as_formula(
                    &sts.database,
                    proof_tree,
                    tree_index,
                    use_provables,
                )?,
                use_provables,
            ),
        }
    }

    fn get_header(&self) -> String {
        match self {
            ExpressionRenderer::Ascii => "".to_string(),
            ExpressionRenderer::Unicode(uni) => uni.get_header(),
            #[cfg(feature = "sts")]
            ExpressionRenderer::Sts(sts) => sts.header.clone(),
        }
    }

    pub fn as_formula(
        database: &Database,
        proof_tree: &ProofTreeArray,
        tree_index: usize,
        use_provables: bool,
    ) -> Result<Formula, String> {
        let formula_string = String::from_utf8_lossy(&proof_tree.exprs().unwrap()[tree_index]);
        let nset = database.name_result();
        let grammar = database.grammar_result();
        let typecodes = if use_provables {
            Box::new([grammar.provable_typecode()])
        } else {
            grammar.typecodes()
        };
        typecodes
            .iter()
            .find_map(|tc| {
                grammar
                    .parse_string(
                        format!("{} {}", as_str(nset.atom_name(*tc)), formula_string.trim())
                            .as_str(),
                        nset,
                    )
                    .ok()
            })
            .ok_or_else(|| format!("{} - Could not parse formula", formula_string))
    }
}

impl Renderer {
    pub(crate) fn new(
        db: Database,
        bib_file: Option<String>,
        #[cfg(feature = "sts")] sts: StsDefinition,
    ) -> Renderer {
        let mut templates = Handlebars::new();
        templates.register_escape_fn(handlebars::no_escape);
        templates
            .register_template_string("statement", include_str!("statement.hbs"))
            .expect("Unable to parse statement template.");
        templates
            .register_template_string("toc", include_str!("toc.hbs"))
            .expect("Unable to parse table of contents template.");
        let contrib_regex = Regex::new(r"\((Contributed|Revised|Modified|Proof[ \n]+shortened)[ \n]+by[ \n]+(?s)(.+?),[ \n]+(\d{1,2}-\w\w\w-\d{4})\.\)").unwrap();
        let discouraged_regex =
            Regex::new(r"\(New usage is discouraged\.\)|\(Proof modification is discouraged\.\)")
                .unwrap();
        let math_regex = Regex::new(r"` (:?[^`]+) `").unwrap();
        let link_regex = Regex::new(r"\~ ([^ \n]+)[ \n]+").unwrap();
        let bibl_regex = Regex::new(r"\[([^ \n]+)\]").unwrap();
        let underline_regex = Regex::new(r"[ \n]_([^_]+)_").unwrap();
        Renderer {
            templates: Arc::new(templates),
            db: db.clone(),
            contrib_regex,
            discouraged_regex,
            link_regex,
            bibl_regex,
            bib_file: bib_file.unwrap_or("".to_string()),
            math_regex,
            underline_regex,
            uni: UnicodeRenderer { database: db },
            #[cfg(feature = "sts")]
            sts,
        }
    }

    fn get_expression_renderer(&self, explorer: String) -> Option<ExpressionRenderer> {
        match explorer.as_str() {
            "mpeascii" => Some(ExpressionRenderer::Ascii),
            "mpeuni" => Some(ExpressionRenderer::Unicode(self.uni.clone())),
            #[cfg(feature = "sts")]
            "mpests" => Some(ExpressionRenderer::Sts(self.sts.clone())),
            _ => None,
        }
    }

    pub(crate) fn get_typesettings() -> Vec<TypesettingInfo> {
        vec![
            TypesettingInfo {
                dir: "mpeascii",
                name: "Ascii",
            },
            TypesettingInfo {
                dir: "mpeuni",
                name: "Unicode",
            },
            #[cfg(feature = "sts")]
            TypesettingInfo {
                dir: "mpests",
                name: "Structured",
            },
        ]
    }

    fn stmt_type(stmt: StatementRef) -> String {
        match stmt.statement_type() {
            StatementType::Provable => "theorem".to_string(),
            StatementType::Axiom => "axiom".to_string(),
            StatementType::Essential => "hyp".to_string(),
            StatementType::Floating => "float".to_string(),
            _ => "other".to_string(),
        }
    }

    pub(crate) fn render_comment(&self, comment: &str) -> String {
        let comment = comment.replace("\n\n", "</p>\n<p>");
        let comment = self.contrib_regex.replace_all(&comment, |caps: &Captures| {
            format!(
                "<span class=\"contrib\">({} by <a href=\"/contributors#{}\">{}</a>, {})</span>",
                caps.get(1)
                    .expect("Contribution Regex did not return a contribution type")
                    .as_str(),
                caps.get(2)
                    .expect("Contribution Regex did not return a contributor")
                    .as_str(),
                caps.get(2)
                    .expect("Contribution Regex did not return a contributor")
                    .as_str(),
                caps.get(3)
                    .expect("Contribution Regex did not return a contribution date")
                    .as_str(),
            )
        });
        let comment = self
            .discouraged_regex
            .replace_all(&comment, |caps: &Captures| {
                format!(
                    "<span class=\"discouraged\">{}</span>",
                    caps.get(0).unwrap().as_str(),
                )
            });
        let comment = self.math_regex.replace_all(&comment, |caps: &Captures| {
            format!(
                "<span class=\"math\">{}</span>",
                caps.get(1).unwrap().as_str()
            )
        });
        let comment = self.link_regex.replace_all(&comment, |caps: &Captures| {
            format!(
                "<a href=\"{}\" class=\"label\">{}</a> ",
                caps.get(1).map_or("#", |m| m.as_str()),
                caps.get(1).map_or("-", |m| m.as_str())
            )
        });
        let comment = comment.replace("~~", "~");
        let comment = self
            .underline_regex
            .replace_all(&comment, |caps: &Captures| {
                format!("<em>{}</em>", caps.get(1).unwrap().as_str(),)
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
    }

    pub(crate) fn render_comment_new(&self, buf: &[u8], span: Span) -> String {
        let mut parser = CommentParser::new(buf, span);
        let mut htmls = 0;
        let mut trim_prev_ws = true;
        let mut comment = "".to_string();
        while let Some(item) = parser.next() {
            let mut out = vec![];
            match item {
                CommentItem::Text(sp) => {
                    out.clear();
                    parser.unescape_text(sp, &mut out);
                    let mut s = std::str::from_utf8(&out).unwrap();
                    if trim_prev_ws {
                        const CLOSING_PUNCTUATION: &str = ".,;)?!:]'\"_-";
                        s = s.trim_start();
                        trim_prev_ws = false;
                        if matches!(s.chars().next(), Some(c) if !CLOSING_PUNCTUATION.contains(c)) {
                            comment.push(' ');
                        }
                    }
                    if htmls == 0 {
                        comment.push_str(s);
                    } else {
                        comment.push_str(
                            &s.replace('&', "&amp;")
                                .replace('<', "&lt;")
                                .replace('>', "&gt;"),
                        );
                    }
                }
                CommentItem::LineBreak(_) => {
                    trim_prev_ws = true;
                    comment.push_str("<p style=\"margin-bottom:0em\">");
                }
                // TODO : math mode: gather symbols, defer to expression renderer
                CommentItem::StartMathMode(_) => {}
                CommentItem::EndMathMode(_) => {}
                CommentItem::MathToken(sp) => {
                    out.clear();
                    parser.unescape_math(sp, &mut out);
                    // TODO! comment.push_str(self.html_defs[&*out]);
                }
                CommentItem::Label(_, sp) => {
                    trim_prev_ws = true;
                    out.clear();
                    parser.unescape_label(sp, &mut out);
                    comment.push_str(&format!(
                        "<a href=\"{label}.html\" class=\"label\">{label}</a>",
                        label = as_str(&out)
                    ));
                }
                CommentItem::Url(_, sp) => {
                    trim_prev_ws = true;
                    out.clear();
                    parser.unescape_label(sp, &mut out);
                    comment.push_str(&format!("<a href=\"{url}\">{url}</a>", url = as_str(&out)));
                }
                CommentItem::StartHtml(_) => htmls += 1,
                CommentItem::EndHtml(_) => htmls -= 1,
                CommentItem::StartSubscript(_) => comment.push_str("<sub><font size=\"-1\">"),
                CommentItem::EndSubscript(_) => comment.push_str("</font></sub>"),
                CommentItem::StartItalic(_) => comment.push_str("<em>"),
                CommentItem::EndItalic(_) => {
                    trim_prev_ws = true;
                    comment.push_str("</em>");
                }
                CommentItem::BibTag(sp) => {
                    trim_prev_ws = false;
                    comment.push_str(&format!(
                        "[<a href=\"{file}#{tag}\">{tag}</a>]",
                        file = self.bib_file,
                        tag = as_str(sp.as_ref(buf))
                    ));
                }
            }
        }
        comment
    }

    pub fn render_statement(&self, explorer: String, label: String) -> Option<String> {
        let sref = self.db.statement(label.as_bytes())?;
        let expression_renderer = self.get_expression_renderer(explorer.clone())?;

        // Header
        let header = expression_renderer.get_header();

        // Table of Contents - Breadcrumb - Prev and Next links
        let nav = self.get_nav(&self.db.get_outline_node(sref));

        // Comments
        let comment = if let Some(cmt) = sref.associated_comment() {
            let mut span = cmt.span();
            span.start += 2;
            span.end -= 3;
            self.render_comment(&String::from_utf8_lossy(
                span.as_ref(&cmt.segment().segment.buffer),
            ))
        } else {
            "(This statement does not have an associated comment)".to_string()
        };

        // Previous and next statements
        // let _prev_label = if let Some(prev_sref) = self.db.prev_sref(sref) { &String::from_utf8_lossy(prev_sref.label()) } else { "" };

        // Proof or Syntax proof
        let (is_proof, steps) = match sref.statement_type() {
            StatementType::Provable => (
                true,
                match self.db.get_proof_tree(sref) {
                    Some(proof_tree) => {
                        proof_tree.with_logical_steps(&self.db, |cur, ix, stmt, hyps| StepInfo {
                            id: ix.to_string(),
                            hyps: hyps.iter().map(usize::to_string).collect::<Vec<String>>(),
                            label: as_str(stmt.label()).to_string(),
                            r#type: Renderer::stmt_type(stmt),
                            link: stmt.is_assertion(),
                            expr: expression_renderer
                                .clone()
                                .render_expression(&proof_tree, cur, true)
                                .unwrap_or_else(|e| {
                                    format!(
                                        "Could not format {} : {}",
                                        &String::from_utf8_lossy(&proof_tree.exprs().unwrap()[cur]),
                                        e
                                    )
                                }),
                        })
                    }
                    None => vec![],
                },
            ),
            StatementType::Axiom | StatementType::Essential | StatementType::Floating => (
                false,
                match self.db.stmt_parse_result().get_formula(&sref) {
                    Some(formula) => {
                        let proof_tree = self.db.get_syntax_proof_tree(formula);
                        proof_tree.with_steps(&self.db, |cur, stmt, hyps| StepInfo {
                            id: cur.to_string(),
                            hyps: hyps.iter().map(usize::to_string).collect::<Vec<String>>(),
                            label: as_str(stmt.label()).to_string(),
                            r#type: Renderer::stmt_type(stmt),
                            link: stmt.is_assertion(),
                            expr: expression_renderer
                                .clone()
                                .render_expression(&proof_tree, cur, false)
                                .unwrap_or_else(|e| {
                                    format!(
                                        "Could not format {} : {}",
                                        &String::from_utf8_lossy(&proof_tree.exprs().unwrap()[cur]),
                                        e
                                    )
                                }),
                        })
                    }
                    None => vec![],
                },
            ),
            _ => (false, vec![]),
        };

        // Statement type
        let statement_type = if is_proof {
            "Theorem".to_string()
        } else if steps.is_empty() {
            "Syntax definition".to_string()
        } else if label.starts_with("df-") {
            "Definition".to_string()
        } else {
            "Axiom".to_string()
        };

        // Statement assertion
        let expr = expression_renderer
            .render_statement(&sref, &self.db, is_proof)
            .unwrap_or_else(|e| format!("Could not format assertion : {}", e));

        // Hypotheses
        let hyps = self
            .db
            .scope_result()
            .get(sref.label())?
            .as_ref(&self.db)
            .essentials()
            .map(|(label, formula)| HypInfo {
                label: as_str(self.db.name_result().atom_name(label)).to_string(),
                expr: expression_renderer
                    .render_formula(formula, &self.db, is_proof)
                    .unwrap_or_else(|e| e),
            })
            .collect();

        let info = PageInfo {
            header,
            nav,
            explorer,
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
