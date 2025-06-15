use crate::config_service::GLOBAL_CONFIG;
use bollard::Docker;
use bollard::container::RemoveContainerOptions;
use std::fs;
use std::path::Path;
use std::process::Command;
use crate::models::cleanup_models::{ActivityType,CleanupService};

pub const CLEANUP_ACTIVITY_CONTAINER: &str = "container";
// pub const CLEANUP_ACTIVITY_IMAGE: &str = "image";
pub const CLEANUP_ACTIVITY_ALL_TARS: &str = "all tars";
// pub const CLEANUP_ACTIVITY_TAR: &str = "tar";


impl CleanupService {
    pub async fn cleanup(&self, activity: ActivityType) -> Result<(), Box<dyn std::error::Error>> {
        println!("Cleaning up Service Called...");
        if let Some(_) = activity.container {
            println!("Cleaning up container...");
            Self::cleanup_containers().await?;
        }
        if let Some(_) = activity.image {
            println!("Cleaning up image...");
            // self.cleanup_images().await?;
        }
        if let Some(_) = activity.all_tars {
            println!("Cleaning up all tar...");
            Self::cleanup_tars().await?;
        }
        if let Some(ref tar_path) = activity.tar {
            println!("Cleaning up tar...");
            Self::cleanup_single_tar(tar_path).await?;
        }
        if let Some(ports) = activity.ports {
            println!("Cleaning up ports...{:?}", ports);
            Self::cleanup_ports(ports).await;
        }
        if activity.container.is_none()
            && activity.image.is_none()
            && activity.all_tars.is_none()
            && activity.tar.is_none()
        {
            println!("No cleanup activity specified.");
        }

        Ok(())
    }

    async fn cleanup_containers() -> Result<(), Box<dyn std::error::Error>> {
        let docker = Docker::connect_with_local_defaults()?;
        let created_by_tag = GLOBAL_CONFIG
            .get()
            .unwrap()
            .constants
            .docker_created_by_label
            .clone();
        let label: String = GLOBAL_CONFIG.get().unwrap().build.service_name.clone();
        let containers = docker
            .list_containers(Some(
                bollard::container::ListContainersOptions::<String>::default(),
            ))
            .await?;
        for container in containers {
            let id = container.id.clone().unwrap();
            if let Some(labels) = &container.labels {
                if labels.get(&created_by_tag) == Some(&label) {
                    docker
                        .remove_container(
                            &id,
                            Some(RemoveContainerOptions {
                                force: true,
                                ..Default::default()
                            }),
                        )
                        .await?;
                    println!("Removed container: {}", id);
                }
            }
        }
        Ok(())
    }

    async fn cleanup_tars() -> Result<(), Box<dyn std::error::Error>> {
        let tar_path_base = &GLOBAL_CONFIG.get().unwrap().paths.tar_path; //returns "./docker/context/"
        for entry in fs::read_dir(tar_path_base.as_ref() as &Path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    async fn cleanup_single_tar(tar_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if Path::new(tar_path).exists() {
            if let Err(e) = std::fs::remove_file(&tar_path) {
                eprintln!("Warning: Failed to delete {}: {}", tar_path, e);
            }
            println!("Deleted tar file: {}", tar_path);
        } else {
            println!("Tar file does not exist: {}", tar_path);
        }
        Ok(())
    }

    async fn cleanup_ports(ports: Vec<i32>) {
        let ports_arg = ports
            .iter()
            .map(|port| port.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        let output = Command::new("./shell_scripts/kill_ports.sh")
            .arg(ports_arg)
            .output()
            .expect("Failed to execute kill_ports.sh");

        if output.status.success() {
            println!("kill_ports.sh executed successfully.");
            println!("Output: {}", String::from_utf8_lossy(&output.stdout));
        } else {
            eprintln!("kill_ports.sh execution failed.");
            eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
        }

        println!("Ports cleaned up: {:?}", ports);
    }
}
