pub struct ValidationService;


#[derive(Debug)]
pub enum ValidationError {
    InvalidLanguage(String),
    EmptyCode(),
    EmptyLanguage(),
    SessionIdError(String),
    InvalidCode(String),
}

pub struct ValidRequest {
    pub session_id: String,
    pub code: String,
    pub language: String,
}