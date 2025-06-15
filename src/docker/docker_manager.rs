use bollard::Docker;
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, StartContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::BuildImageOptions;
use bollard::models::{HostConfig, PortBinding};
use futures_util::stream::StreamExt;
use std::error::Error;
use uuid::Uuid;
use std::str::FromStr;
use tokio::io::AsyncReadExt;

use crate::models::{cleanup_models::{ActivityType, CleanupService}, docker_models::DockerSupportedLanguage};
use crate::config_service::GLOBAL_CONFIG;
use crate::language_executor::generate_shell_command;
use crate::session_management_service::SessionManagement;
use crate::utils::{docker_utils::get_docker_instance, tar_utils::create_tar_archive};
use crate::models::validation_models::ValidationError;

pub async fn handle_request(
    session_id: &str,
    language: &str,
    code: &str,
) -> Result<String, Box<dyn Error>> {
    let docker_language = match DockerSupportedLanguage::from_str(language) {
        Ok(lang) => lang,
        Err(_) => {
            eprintln!("Unsupported language: {}", language);
            return Err(Box::new(ValidationError::InvalidLanguage(
                language.to_string(),
            )));
        }
    };
    let docker = get_docker_instance()?;
    //Docker::connect_with_local_defaults()?;
    let config = GLOBAL_CONFIG.get().unwrap();
    // Select the appropriate Dockerfile
    let dockerfile_path = match DockerSupportedLanguage::from_str(language) {
        Ok(DockerSupportedLanguage::Python) => &config.dockerfiles.python,
        Ok(DockerSupportedLanguage::JavaScript) => &config.dockerfiles.javascript,
        Ok(DockerSupportedLanguage::Java) => &config.dockerfiles.java,
        // Ok(DockerSupportedLanguage::Go) => &config.dockerfiles.go,
        _ => return Err(format!("Unsupported language: {}", language).into()),
    };

    // Build and run the container
    let container_name =
        build_and_run_container(session_id, &docker, dockerfile_path, language).await?;

    // Execute the code inside the container
    let result =
        execute_code_in_new_container(&docker, &container_name, docker_language, code).await?;

    Ok(result)
}

pub async fn build_and_run_container(
    session_id: &str,
    docker: &Docker,
    dockerfile_path: &str,
    language: &str,
) -> Result<String, Box<dyn Error>> {
    println!("Building and running container for language: {}", language);
    let config = GLOBAL_CONFIG.get().unwrap();
    let image_name = format!(
        "{}_{}_{}",
        config.constants.executor_image_name, session_id, language
    );
    // Create tar archive for build context

    let tar_path_base = &config.paths.tar_path; //returns "./docker/context/"
    // println!("tar_path_base: {}", tar_path_base);
    let ref tar_path_formatted = format!(
        "{}{}_{}_{}",
        tar_path_base,
        Uuid::new_v4(),
        language,
        &config.constants.tar_file_name
    );
    let docker_file_name = &config.constants.dockerfile;
    let dockerfile_name =
        create_tar_archive(dockerfile_path, &tar_path_formatted, docker_file_name)?;
    println!("Using dockerfile_name: '{}'", dockerfile_name);
    // Use a sync File, not tokio::fs::File, because bollard expects a blocking Read stream
    let mut file = tokio::fs::File::open(tar_path_formatted).await?;

    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;
    // Build image options
    let build_options = BuildImageOptions {
        dockerfile: dockerfile_name,
        t: image_name.clone(),
        rm: true,
        ..Default::default()
    };

    // Start the image build stream
    let mut build_stream = docker.build_image(build_options, None, Some(contents.into()));

    // Print docker build output logs
    while let Some(build_output) = build_stream.next().await {
        match build_output {
            Ok(output) => {
                if let Some(stream) = output.stream {
                    print!("{}", stream);
                }
            }
            Err(e) => {
                eprintln!("Error during image build: {}", e);
                return Err(Box::new(e));
            }
        }
    }

    println!("Docker image '{}' built successfully!", image_name);

    // clear the tar async from tar_path_formatted
    let activity_to_clear_tar =
        ActivityType::new(None, None, None, Some(tar_path_formatted.to_string()), None);
    let cleanup_service = CleanupService {};
    // Spawn a new task to clean up the tar file asynchronously, don't await
    tokio::spawn(async move {
        if let Err(e) = cleanup_service.cleanup(activity_to_clear_tar).await {
            eprintln!("Failed to clean up tar file: {}", e);
        } else {
            println!("Tar file cleaned up successfully.");
        }
    });

    // Create container config

    let container_name = format!(
        "{}_{}_{}",
        GLOBAL_CONFIG
            .get()
            .unwrap()
            .constants
            .executor_container_name,
        language,
        session_id
    );
    let created_by_tag = GLOBAL_CONFIG
        .get()
        .unwrap()
        .constants
        .docker_created_by_label
        .clone();
    let label: String = GLOBAL_CONFIG.get().unwrap().build.service_name.clone();

    let config = ContainerConfig {
        labels: Some([(created_by_tag, label)].iter().cloned().collect()),
        image: Some(image_name.clone()),
        host_config: Some(HostConfig {
            port_bindings: Some(
                [(
                    "5001/tcp".to_string(),
                    Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some("5001".to_string()),
                    }]),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };
    // Create container
    docker
        .create_container(
            Some(CreateContainerOptions {
                name: &container_name,
                platform: None,
            }),
            config,
        )
        .await?;
    println!("Container '{}' created successfully.", container_name);

    // Start container
    docker
        .start_container(&container_name, None::<StartContainerOptions<String>>)
        .await?;
    println!("Container '{}' started successfully!", container_name);

    // Store session info
    let session_service = &GLOBAL_CONFIG
        .get()
        .expect("Global config not set")
        .session_management_service;

    session_service
        .add_session(
            session_id.to_string(),
            language.to_string(),
            container_name.clone(),
        )
        .await
        .map_err(|e| format!("Failed to save session: {}", e.message()))?;
    println!(
        "Session stored successfully for ID '{}', language '{}'",
        session_id, language
    );
    // FOR TESTING PURPOSES: Retrieve and print session image
    match session_service
        .get_session_image(session_id, language)
        .await
    {
        Ok(image) => {
            println!("Session image for {}: {}", session_id, image);
        }
        Err(e) => {
            eprintln!("Error retrieving session image: {:?}", e);
        }
    }
    Ok(container_name)
}

async fn execute_code_in_new_container(
    docker: &Docker,
    container_name: &str,
    language: DockerSupportedLanguage,
    code: &str,
) -> Result<String, Box<dyn Error>> {
    let shell_command = generate_shell_command(language, code)
        .map_err(|e| format!("Failed to generate shell command: {}", e))?; //format!("echo '{}' > script.py && python script.py", code);
    let exec_options = CreateExecOptions {
        cmd: Some(vec!["sh", "-c", &shell_command]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        ..Default::default()
    };

    let exec = docker.create_exec(container_name, exec_options).await?;
    let output = docker.start_exec(&exec.id, None).await?;

    match output {
        StartExecResults::Attached { mut output, .. } => {
            let mut result = String::new();
            while let Some(Ok(log)) = output.next().await {
                match log {
                    bollard::container::LogOutput::StdOut { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        _ => Err("Failed to execute code in container".into()),
    }
}

/// Executes code in an existing, already running container.
/// You can call this function with the container name/id and code to execute.
///
/// # Arguments
/// * `docker` - Reference to the Docker client
/// * `container_name` - Name or ID of the running container
/// * `code` - The code to execute inside the container
///
/// # Returns
/// * `Result<String, Box<dyn Error>>` - Output from the code execution or error
pub async fn execute_code_in_existing_container(
    container_name: &str,
    language: DockerSupportedLanguage,
    code: &str,
) -> Result<String, Box<dyn Error>> {
    let docker = get_docker_instance()?;
    // update this to accept multiple langeages
    let shell_command = generate_shell_command(language, code)
        .map_err(|e| format!("Failed to generate shell command: {}", e))?;
    println!(
        "Executing code in existing container '{}': {}",
        container_name, shell_command
    );
    let exec_options = CreateExecOptions {
        cmd: Some(vec!["sh", "-c", &shell_command]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        ..Default::default()
    };

    let exec = docker.create_exec(container_name, exec_options).await?;
    let output = docker.start_exec(&exec.id, None).await?;

    match output {
        StartExecResults::Attached { mut output, .. } => {
            let mut result = String::new();
            while let Some(Ok(log)) = output.next().await {
                match log {
                    bollard::container::LogOutput::StdOut { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        _ => Err("Failed to execute code in existing container".into()),
    }
}
