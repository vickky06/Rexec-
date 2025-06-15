use crate::config_service::GLOBAL_CONFIG;
use crate::models::docker_models::DockerSupportedLanguage;
use crate::proto::executor::ExecuteRequest;
use crate::models::session_management_models::{SessionError};
use crate::session_management_service::SessionManagement;
use crate::models::validation_models::{ValidationService, ValidRequest, ValidationError};

use std::error::Error;
use std::fmt;
use tonic::Request;


impl ValidRequest {
    pub fn new(id: String, code: String, language: String) -> Self {
        ValidRequest {
            session_id: id,
            code: code,
            language: language,
        }
    }
    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    pub fn get_code(&self) -> &str {
        &self.code
    }

    pub fn get_language(&self) -> &str {
        &self.language
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation Error : {:?}", self)
    }
}

impl Error for ValidationError {}
impl ValidationError {
    fn to_string(&self) -> String {
        match self {
            ValidationError::InvalidLanguage(lang) => {
                SessionError::message(&SessionError::InvalidLanguage(lang.clone()))
            }
            ValidationError::InvalidCode(code) => format!("Invalid Code {}", code),
            ValidationError::EmptyCode() => format!("Code must be provided:"),
            ValidationError::EmptyLanguage() => format!("Language must be specified"),
            ValidationError::SessionIdError(msg) => format!("Session ID error: {}", msg),
        }
    }
}


impl ValidationService {
    pub async fn validate_request(
        request: &Request<ExecuteRequest>,
    ) -> Result<ValidRequest, ValidationError> {
        // Extract the session_id from metadata before moving request
        let session_id = match GLOBAL_CONFIG
            .get()
            .unwrap()
            .session_management_service
            .get_session_id(request)
        {
            Ok(id) => id,
            Err(e) => {
                let error = ValidationError::SessionIdError(format!("{:?}", e));
                println!("{}", error.to_string());
                return Err(error);
            }
        };

        // Now borrow the request data
        let request_data = request.get_ref();
        println!("Received request: {:?}", request_data);
        let language = request_data.language.to_lowercase();
        let code = request_data.code.clone();
        if language.is_empty() {
            return Err(ValidationError::EmptyLanguage());
        }
        let docker_language = DockerSupportedLanguage::is_supported(&language);

        if docker_language.is_none() {
            return Err(ValidationError::InvalidLanguage(format!("{:?}", language)));
        } else {
            println!(
                "Language is valid: {}",
                DockerSupportedLanguage::to_string(&docker_language.unwrap())
            );
        }

        if code.is_empty() {
            eprint!("{:?}", ValidationError::EmptyCode());
            return Err(ValidationError::EmptyCode());
        }

        return Ok(ValidRequest::new(session_id, code, language));
    }
}
