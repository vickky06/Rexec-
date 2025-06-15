use crate::models::port_models::PortsService;
use crate::config_service::GLOBAL_CONFIG;

impl PortsService {
    pub fn new() -> Self {
        let build = &GLOBAL_CONFIG.get().unwrap().build;
        PortsService {
            grpc_server_port: build.service_port,
            grpc_ui_port: build.grpc_ui_port,
            websocket_port: build.web_socket_port,
            host: build.host.clone(),
        }
    }

    pub fn get_grpc_server_address(&self) -> String {
        format!("{}{}", self.host, self.grpc_server_port)
    }
    pub fn get_grpc_ui_address(&self) -> String {
        format!("{}{}", self.host, self.grpc_ui_port)
    }
    pub fn get_websocket_address(&self) -> String {
        format!("{}{}", self.host, self.websocket_port)
    }

    pub fn get_all_ports(&self) -> Vec<i32> {
        vec![
            self.grpc_server_port,
            self.grpc_ui_port,
            self.websocket_port,
        ]
    }
}
