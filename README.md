# Descartes

AI agent orchestration using the Swarm pattern - fresh context per task to prevent drift and error accumulation.

**[Documentation](https://pyrex41.github.io/descartes/)** | [Getting Started](https://pyrex41.github.io/descartes/getting-started.html) | [Configuration](https://pyrex41.github.io/descartes/configuration.html)

## Overview

Descartes implements wave-based task execution with SCUD (DAG-driven task management):

```
PRD → SCUD Tasks → Waves → Fresh Agent Per Task → Validation
```

## Quick Start

```bash
# Install
cd descartes && cargo install --path .

# Initialize in your project
descartes init

# Execute tasks from a PRD
descartes swarm --prd ./docs/feature.md --tag my-feature --verify "cargo test"
```

## Repository Structure

```
descartes/          # Main CLI crate
descartes-gui/      # Optional GUI (published to crates.io)
docs/               # GitHub Pages documentation source
```

## License

MIT
