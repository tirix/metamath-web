//! Unicode Typesetting
use metamath_knife::formula::Formula;
use metamath_knife::statement::as_str;
use metamath_knife::statement::StatementRef;
use metamath_knife::Database;
use std::fmt::Write;

#[derive(Clone)]
pub(crate) struct UnicodeRenderer {
    pub(crate) database: Database,
}

impl UnicodeRenderer {
    pub(crate) fn get_header(&self) -> String {
        //        self.database.typesetting_result().html_css.as_ref().map_or("", |t| as_str(&t)).to_string()
        "".into()
    }

    pub(crate) fn render_formula(&self, formula: &Formula) -> Result<String, String> {
        let mut output: String = "<span class=\"uni\"><span color=\"gray\">⊢</span> ".into();
        //        write!(output, "{} ", as_str(if use_provables
        let typesetting = self.database.typesetting_result();
        let nset = self.database.name_result();
        for symbol in formula.as_ref(&self.database).into_iter() {
            let token = nset.atom_name(symbol);
            write!(
                output,
                "{} ",
                as_str(
                    typesetting
                        .get_alt_html_def(token)
                        .ok_or(format!("Unknown symbol: {}", as_str(token)).to_string())?
                )
            )
            .unwrap();
        }
        write!(output, "</span>").unwrap();
        Ok(output)
    }

    pub(crate) fn render_statement(&self, sref: &StatementRef) -> Result<String, String> {
        let mut output: String = "<span class=\"uni\">".into();
        let typesetting = self.database.typesetting_result();
        for token in sref.math_iter() {
            write!(
                output,
                "{} ",
                as_str(
                    typesetting
                        .get_alt_html_def(&token)
                        .ok_or(format!("Unknown symbol: {}", as_str(&token)).to_string())?
                )
            )
            .unwrap();
        }
        write!(output, "</span>").unwrap();
        Ok(output)
    }
}
