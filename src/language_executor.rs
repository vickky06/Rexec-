use std::error::Error;

use crate::models::docker_models::DockerSupportedLanguage;

pub fn generate_shell_command(
    language: DockerSupportedLanguage,
    code: &str,
) -> Result<String, Box<dyn Error>> {
    match language {
        DockerSupportedLanguage::Python => {
            Ok(format!("echo '{}' > script.py && python script.py", code))
        }
        DockerSupportedLanguage::JavaScript => {
            Ok(format!("echo '{}' > script.js && node script.js", code))
        }
        DockerSupportedLanguage::Java => Ok(format!(
            "echo '{}' > Main.java && javac Main.java && java Main",
            code
        )),
        // _ => Err(format!("Unsupported language: {:?}",DockerSupportedLanguage::to_string(&language)).into()),
    }
}
