use std::str::FromStr;

use crate::config_service::GLOBAL_CONFIG;
use crate::docker::docker_manager;
use crate::models::docker_models::DockerSupportedLanguage;
use crate::proto::executor::code_executor_server::CodeExecutor;
use crate::proto::executor::{ExecuteRequest, ExecuteResponse};
use crate::session_management_service::SessionManagement;
use crate::models::validation_models::{ValidRequest, ValidationError, ValidationService};
use tonic::{Request, Response, Status};
use crate::models::executor_models::ExecutorService;

#[tonic::async_trait]
impl CodeExecutor for ExecutorService {
    async fn execute(
        &self,
        request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteResponse>, Status> {
        let valid_data = match ValidationService::validate_request(&request).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Validation error: {:?}", e);
                return Err(Status::invalid_argument(format!(
                    "Validation error: {:?}",
                    e
                )));
            }
        };
        match session_handler(valid_data).await {
            Ok(output) => {
                println!("Execution Result: {}", output);
                Ok(Response::new(ExecuteResponse { message: output }))
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                Err(Status::internal(format!("Execution error: {}", e)))
            }
        }
    }
}

pub async fn session_handler(data: ValidRequest) -> Result<String, Box<dyn std::error::Error>> {
    let session_id = data.get_session_id();
    let language = data.get_language();
    let language_str = language.to_string();
    let code = data.get_code();
    println!("Handling request for language: {}", language);

    match GLOBAL_CONFIG
        .get()
        .unwrap()
        .session_management_service
        .get_session_image(session_id, &language_str)
        .await
    {
        Ok(image) => {
            println!("Session image for {}: {}", session_id, image);
            let language = match DockerSupportedLanguage::from_str(language) {
                Ok(lang) => lang,
                Err(_) => {
                    eprintln!("Unsupported language: {}", language);
                    return Err(Box::new(ValidationError::InvalidLanguage(
                        language.to_string(),
                    )));
                }
            };
            match docker_manager::execute_code_in_existing_container(&image, language, code).await {
                Ok(result) => {
                    println!("Execution Result: {}", result);
                    Ok(result)
                }
                Err(e) => {
                    eprintln!("Error executing code in container: {:?}", e);
                    Err(e)
                }
            }
        }

        Err(e) => {
            eprintln!("image not found {:?}", e);
            let result = docker_manager::handle_request(session_id, language, code).await?;
            Ok(result)
        }
    }
}
