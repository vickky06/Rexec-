use super::validator::{SyntaxValidator, ValidationError};
use tree_sitter::Parser;
use tree_sitter_javascript;

// unsafe extern "C" {
//     fn tree_sitter_javascript() -> Language;
// }

pub struct JavaScriptValidator;

impl SyntaxValidator for JavaScriptValidator {
    fn validate(&self, code: &str) -> Result<tree_sitter::Tree, ValidationError> {
        let language = tree_sitter_javascript::language();
        let mut parser = Parser::new();
        println!("{} code received", code);
        parser
            .set_language(language)
            .map_err(|e| self.validation_error(&format!("Set lang error: {:?}", e)))?;

        let tree = parser
            .parse(code, None)
            .ok_or(self.validation_error("Failed to parse Python code"))?;

        let root_node = tree.root_node();
        if root_node.has_error() {
            let error_message = format!(
                "Syntax error detected in code at byte range {:?}",
                root_node.to_sexp()
            );
            return Err(self.validation_error(&error_message));
        }

        Ok(tree)
    }
}
