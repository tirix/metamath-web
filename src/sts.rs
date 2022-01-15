use metamath_knife::formula::TypeCode;
use metamath_knife::formula::Label;
use metamath_knife::Database;
use metamath_knife::Formula;
use metamath_knife::proof::ProofTreeArray;
use metamath_knife::parser::as_str;
use metamath_knife::parser::StatementRef;
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct StsScheme {
    is_identifier: bool,
    typecode: TypeCode,
    formula: Formula ,
    subst: String,
}

impl StsScheme {
    pub fn new(is_identifier: bool, typecode: TypeCode, formula: Formula, subst: &str) -> Self {
        Self  {
            is_identifier,
            typecode,
            formula,
            subst: subst.trim().to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StsDefinition {
    database: Database,
    schemes: Arc<HashMap<TypeCode, Vec<StsScheme>>>,
    identifiers: Arc<HashMap<Label, TypeCode>>, 
    pub(crate) header: String,
    display: String,
    _inline: String,
    _command: String,
}

impl StsDefinition {
    pub fn new(database: Database, schemes_list: Vec<StsScheme>, _header: String, display: String, _inline: String, _command: String) -> Result<Self, String> {
        let mut schemes = HashMap::new();
        let mut identifiers = HashMap::new();
        for ref scheme in schemes_list { 
            if scheme.is_identifier {
                let label = scheme.formula.get_by_path(&[]).ok_or("Empty identifier formula!")?;
                identifiers.insert(label, scheme.typecode);
            }
            schemes.entry(scheme.typecode).or_insert_with(Vec::new).push(scheme.clone());
        }
        let schemes = Arc::new(schemes);
        let identifiers = Arc::new(identifiers);
        let header = "<script src=\"https://polyfill.io/v3/polyfill.min.js?features=es6\"></script>
    		<script id=\"MathJax-script\" async src=\"https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js\"></script>".to_string();
        Ok(Self { database, schemes, identifiers, header, display, _inline, _command })
    }

    fn apply_scheme(&self, scheme: &StsScheme, formula: &Formula) -> Option<String> {
        let nset = self.database.name_result();
        if scheme.is_identifier {
            (&scheme.formula == formula).then(|| scheme.subst.clone())
        } else if let Some(subst) = formula.unify(&scheme.formula) {
            let mut formatted_string = scheme.subst.clone();
            for (label, subformula) in &*subst {
                let sref = self.database.statement_by_label(*label)?;
                let variable_atom = nset.var_atom(sref)?;
                let variable_token = as_str(nset.atom_name(variable_atom));
                let subformula_typecode = self.identifiers.get(&label)?;
                let formatted_substring = self.format(*subformula_typecode, subformula).ok()?;
                let pattern = format!("#{}#", variable_token).to_string();
                formatted_string = formatted_string.replace(&pattern, &formatted_substring);
            }
            Some(formatted_string)
        } else {
            None
        }
    }

    /// Recursively format the given formula, for the given typecode
    fn format(&self, typecode: TypeCode, formula: &Formula) -> Result<String, String> {
        let nset = self.database.name_result();
        for scheme in self.schemes.get(&typecode).ok_or_else(|| format!("No typesetting found for typecode {:?}", typecode))? {
            match self.apply_scheme(scheme, formula) {
                Some(formatted_string) => { return Ok(formatted_string) },
                None => { },
            }
        }
        Err(format!("No typesetting found for {} with typecode {}", formula.as_ref(&self.database), as_str(nset.atom_name(typecode))))
    }

    pub fn render_formula(&self, formula: &Formula, use_provables: bool) -> Result<String, String> {
        let grammar = self.database.grammar_result();
        let typecode = if use_provables { grammar.provable_typecode() } else { formula.get_typecode() };
        let display = self.display.clone();
        Ok(display.replace("###", &self.format(typecode, &formula)?))
    }

    pub fn render_statement(&self, sref: &StatementRef, use_provables: bool) -> Result<String, String> {
        let formula = self.database.stmt_parse_result().get_formula(sref).ok_or("Unknown statement")?;
        self.render_formula(formula, use_provables)
    }

    pub fn render_expression(self, proof_tree: &ProofTreeArray, tree_index: usize, use_provables: bool) -> Result<String, String> {
        let formula_string = String::from_utf8_lossy(&proof_tree.exprs[tree_index]);
        let nset = self.database.name_result();
        let grammar = self.database.grammar_result();
        let typecodes = grammar.typecodes();
        let formula = grammar.parse_formula(
            &mut formula_string.trim().split(" ").map(|t| {
                nset.lookup_symbol(t.as_bytes()).unwrap().atom
            }), 
            &typecodes, 
            nset
        ).map_err(|diag| format!("{} - Could not parse formula: {:?}", formula_string, diag))?;
        self.render_formula(&formula, use_provables)
    }
}