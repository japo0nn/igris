#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Starting IGRIS backend server..."
cd "$SCRIPT_DIR"
./target/debug/igris --server &
BACKEND_PID=$!

sleep 1

echo "Starting IGRIS UI..."
cd "$SCRIPT_DIR/ui"
npm run dev &
FRONTEND_PID=$!

echo ""
echo "====================================="
echo "  IGRIS UI: http://localhost:5173"
echo "  IGRIS API: http://localhost:3001"
echo "====================================="
echo "Press Ctrl+C to stop both servers"

wait
