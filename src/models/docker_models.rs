use std::str::FromStr;

pub enum DockerSupportedLanguage {
    Python,
    JavaScript,
    Java,
    // Go,
}

impl FromStr for DockerSupportedLanguage {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "python" => Ok(DockerSupportedLanguage::Python),
            "javascript" => Ok(DockerSupportedLanguage::JavaScript),
            "java" => Ok(DockerSupportedLanguage::Java),
            // "go" => Ok(DockerSupportedLanguage::Go),
            _ => Err(()),
        }
    }
}
impl DockerSupportedLanguage {
    pub fn is_supported(lang: &str) -> Option<DockerSupportedLanguage> {
        DockerSupportedLanguage::from_str(lang).ok()
    }

    pub fn to_string(language: &DockerSupportedLanguage) -> String {
        match language {
            DockerSupportedLanguage::Python => "python".to_string(),
            DockerSupportedLanguage::JavaScript => "javascript".to_string(),
            DockerSupportedLanguage::Java => "java".to_string(),
            // DockerSupportedLanguage::Go => "go".to_string(),
            // _ => "unknown".to_string(),
        }
    }
}
