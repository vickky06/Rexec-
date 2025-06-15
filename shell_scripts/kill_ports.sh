#!/bin/bash
SHELL_SCRIPTS_PATH="./shell_scripts"
echo "Using shell scripts path: $SHELL_SCRIPTS_PATH"
# Get the ports from get_config.sh
CONFIG_PORTS=($($SHELL_SCRIPTS_PATH/get_config.sh))

# Combine the ports from get_config.sh with the ports passed as arguments
PORTS=("${CONFIG_PORTS[@]}" "$@")
# Check if no ports are provided
if [ "${#PORTS[@]}" -eq 0 ]; then
    echo "No ports provided. Exiting."
    exit 1
fi
# Loop through the ports and check if they are in use
for PORT in "${PORTS[@]}"; do
    echo "Checking port: $PORT"
    if [ -z "$PORT" ]; then
        continue
    fi
    # Find the PID(s) of processes using the port
    PIDS=$(lsof -t -i :"$PORT")
    
    if [ -n "$PIDS" ]; then
        echo "Port $PORT is in use by PID(s): $PIDS"
        # Kill the processes
        echo "$PIDS" | xargs -n 1 kill -9
        echo "Killed PID(s) using port $PORT"
    else
        echo "Port $PORT is not in use."
    fi
done