use metamath_knife::formula::Label;
use metamath_knife::formula::Substitutions;
use metamath_knife::formula::TypeCode;
use metamath_knife::grammar::FormulaToken;
use metamath_knife::statement::as_str;
use metamath_knife::statement::StatementRef;
use metamath_knife::Database;
use metamath_knife::Formula;
use metamath_knife::StatementType;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct StsScheme {
    is_identifier: bool,
    typecode: TypeCode,
    formula: Formula,
    subst: String,
}

impl StsScheme {
    pub fn new(is_identifier: bool, typecode: TypeCode, formula: Formula, subst: &str) -> Self {
        Self {
            is_identifier,
            typecode,
            formula,
            subst: subst.trim().to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StsDefinition {
    pub(crate) database: Database,
    schemes: Arc<HashMap<TypeCode, Vec<StsScheme>>>,
    identifiers: Arc<HashMap<Label, TypeCode>>,
    pub(crate) header: String,
    display: String,
    _inline: String,
    _command: String,
}

impl StsDefinition {
    pub fn new(
        database: Database,
        schemes_list: Vec<StsScheme>,
        _header: String,
        display: String,
        _inline: String,
        _command: String,
    ) -> Result<Self, String> {
        let mut schemes = HashMap::new();
        let mut identifiers = HashMap::new();
        for ref scheme in schemes_list {
            if scheme.is_identifier {
                let label = scheme
                    .formula
                    .get_by_path(&[])
                    .ok_or("Empty identifier formula!")?;
                identifiers.insert(label, scheme.typecode);
            }
            schemes
                .entry(scheme.typecode)
                .or_insert_with(Vec::new)
                .push(scheme.clone());
        }
        let schemes = Arc::new(schemes);
        let identifiers = Arc::new(identifiers);
        let header = "<script src=\"https://polyfill.io/v3/polyfill.min.js?features=es6\"></script>
    		<script id=\"MathJax-script\" async src=\"https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js\"></script>".to_string();
        Ok(Self {
            database,
            schemes,
            identifiers,
            header,
            display,
            _inline,
            _command,
        })
    }

    fn apply_scheme(&self, scheme: &StsScheme, formula: &Formula) -> Option<String> {
        let nset = self.database.name_result();
        if scheme.is_identifier {
            (&scheme.formula == formula).then(|| scheme.subst.clone())
        } else {
            let mut subst = Substitutions::new();
            formula
                .unify(&scheme.formula, &mut subst)
                .ok()
                .and_then(|()| {
                    let mut formatted_string = scheme.subst.clone();
                    for (label, subformula) in &subst {
                        let sref = self.database.statement_by_label(*label)?;
                        let variable_atom = nset.var_atom(sref)?;
                        let variable_token = as_str(nset.atom_name(variable_atom));
                        let subformula_typecode = self.identifiers.get(label)?;
                        let formatted_substring =
                            self.format(*subformula_typecode, subformula).ok()?;
                        let pattern = format!("#{}#", variable_token).to_string();
                        formatted_string = formatted_string.replace(&pattern, &formatted_substring);
                    }
                    Some(formatted_string)
                })
        }
    }

    /// Recursively format the given formula, for the given typecode
    fn format(&self, typecode: TypeCode, formula: &Formula) -> Result<String, String> {
        let nset = self.database.name_result();
        for scheme in self
            .schemes
            .get(&typecode)
            .ok_or_else(|| format!("No typesetting found for typecode {:?}", typecode))?
        {
            if let Some(formatted_string) = self.apply_scheme(scheme, formula) {
                return Ok(formatted_string);
            }
        }
        Err(format!(
            "No typesetting found for {} with typecode {}",
            formula.as_ref(&self.database),
            as_str(nset.atom_name(typecode))
        ))
    }

    pub fn render_formula(&self, formula: &Formula, use_provables: bool) -> Result<String, String> {
        let grammar = self.database.grammar_result();
        let typecode = if use_provables {
            grammar.provable_typecode()
        } else {
            formula.get_typecode()
        };
        let display = self.display.clone();
        Ok(display.replace("###", &self.format(typecode, formula)?))
    }

    pub fn render_statement(
        &self,
        sref: &StatementRef,
        use_provables: bool,
    ) -> Result<String, String> {
        let formula = self
            .database
            .stmt_parse_result()
            .get_formula(sref)
            .ok_or("Unknown statement")?;
        self.render_formula(formula, use_provables)
    }

    pub fn check(&self) {
        let provable = self.database.grammar_result().provable_typecode();
        let nset = self.database.name_result();
        for sref in self.database.statements() {
            if sref.statement_type() == StatementType::Axiom {
                let mut tokens = sref.math_iter();
                let typecode = nset.get_atom(&tokens.next().unwrap());
                if typecode != provable {
                    let formula = self
                        .database
                        .grammar_result()
                        .parse_formula(
                            &mut tokens.map(|t| {
                                Ok(FormulaToken {
                                    symbol: nset.get_atom(&t),
                                    span: metamath_knife::Span::NULL,
                                })
                            }),
                            &[typecode],
                            false,
                            nset,
                        )
                        .unwrap();
                    if let Err(error) = self.format(typecode, &formula) {
                        eprintln!("{}", error);
                    }
                }
            }
        }
    }
}
