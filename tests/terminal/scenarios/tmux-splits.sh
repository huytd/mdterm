#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Split right pane
tmux split-window -h "/bin/bash \"$SCRIPT_DIR/colors.sh\""

# Split bottom-right pane
tmux split-window -v "/bin/bash \"$SCRIPT_DIR/box-drawing.sh\""

# Run box-drawing in the initial (left) pane
/bin/bash "$SCRIPT_DIR/box-drawing.sh"
