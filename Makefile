SHELL_SCRIPTS_PATH = ./shell_scripts

build:
	cargo build

run:
	$(eval PORTS=$(shell $(SHELL_SCRIPTS_PATH)/get_config.sh))
	$(eval SERVICE_PORT=$(word 1, $(PORTS)))
	@echo "SERVICE_PORT=$(SERVICE_PORT)"
	cargo run -- run

clean:
	$(eval PORTS=$(shell $(SHELL_SCRIPTS_PATH)/get_config.sh))
	$(eval SERVICE_PORT=$(word 1, $(PORTS)))
	$(eval GRPC_UI_PORT=$(word 2, $(PORTS)))
	@echo "SERVICE_PORT=$(SERVICE_PORT)"
	@echo "GRPC_UI_PORT=$(GRPC_UI_PORT)"
	@echo "Starting grpcui on GRPC_UI_PORT=$(GRPC_UI_PORT) and SERVICE_PORT=$(SERVICE_PORT)"

fmt:
	cargo fmt

grpcui:
	cargo run -- run-grpcui &
	sleep 10
	$(eval PORTS=$(shell $(SHELL_SCRIPTS_PATH)/get_config.sh))
	$(eval SERVICE_PORT=$(word 1, $(PORTS)))
	$(eval GRPC_UI_PORT=$(word 2, $(PORTS)))
	$(eval WEBSOCKET_PORT=$(word 3, $(PORTS)))
	@echo "Starting grpcui on GRPC_UI_PORT=$(GRPC_UI_PORT) and SERVICE_PORT=$(SERVICE_PORT) AND WEBSOCKET_PORT=$(WEBSOCKET_PORT)"
	grpcui -plaintext -port $(GRPC_UI_PORT) localhost:$(SERVICE_PORT)

all: fmt build test

kill_ports:
	$(SHELL_SCRIPTS_PATH)/kill_ports.sh
