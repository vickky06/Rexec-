use serde::Deserialize;
use crate::models::session_management_models::SessionManagementService;

#[derive(Debug, Deserialize, Clone)]
pub struct Dockerfiles {
    pub python: String,
    pub javascript: String,
    pub java: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paths {
    pub tar_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Constants {
    pub dockerfile: String,
    pub docker_created_by_label: String,
    pub service_name: String,
    pub executor_container_name: String,
    pub executor_image_name: String,
    pub tar_file_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Build {
    pub service_port: i32,
    pub service_name: String,
    pub grpc_ui_port: i32,
    pub web_socket_port: i32,
    pub host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SessionConfigs {
    pub session_timeout: u64,
    pub session_cleanup_interval: u64,
    pub max_sessions: usize,
    // pub session_image_prefix: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub dockerfiles: Dockerfiles,
    pub paths: Paths,
    pub constants: Constants,
    pub build: Build,
    pub session_configs: SessionConfigs,
    #[serde(skip)]
    pub session_management_service: SessionManagementService,
}
