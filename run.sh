#! /bin/bash
export NAME="cb/run"

sudo service redis-server start
sudo service postgresql start

cd api-service
tmux new-session -s $NAME -d "env RUST_LOG=debug cargo run; bash"
cd ../frontend
tmux split-window -h "yarn workspace @frontend/desktop dev; bash"
tmux split-window -v "yarn workspace @frontend/desktop check-ts --watch; bash"

tmux -2 attach-session -d
