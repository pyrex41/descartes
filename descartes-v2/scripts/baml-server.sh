#!/bin/bash
# Start the BAML development server
#
# This script starts the BAML HTTP server that exposes BAML functions as REST endpoints.
# The Rust code in src/baml/mod.rs calls these endpoints.
#
# Requirements:
#   - Node.js and npm installed
#   - ANTHROPIC_API_KEY and/or OPENAI_API_KEY environment variables set
#
# Usage:
#   ./scripts/baml-server.sh        # Start server
#   ./scripts/baml-server.sh --help # Show help
#
# Server endpoints:
#   http://localhost:2024/_debug/ping  - Health check
#   http://localhost:2024/docs         - Swagger UI
#   http://localhost:2024/call/<func>  - Call BAML function

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BAML_DIR="$PROJECT_DIR/baml_src"

# Check for required environment variables
if [[ -z "$ANTHROPIC_API_KEY" && -z "$OPENAI_API_KEY" ]]; then
    echo "Warning: Neither ANTHROPIC_API_KEY nor OPENAI_API_KEY is set"
    echo "BAML functions that require these providers will fail"
fi

# Check for Node.js
if ! command -v npx &> /dev/null; then
    echo "Error: npx not found. Please install Node.js"
    exit 1
fi

# Change to baml_src directory
cd "$BAML_DIR"

echo "Starting BAML server..."
echo "  Directory: $BAML_DIR"
echo "  Server: http://localhost:2024"
echo "  Docs: http://localhost:2024/docs"
echo ""

# Start the BAML dev server
exec npx @boundaryml/baml dev --preview
