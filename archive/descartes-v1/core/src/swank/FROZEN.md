# FROZEN FEATURE

This feature is complete and functional but not under active development.
It enables AI agents to do live Common Lisp development with SBCL.

Status: Frozen as of 2026-01-09
Reason: Specialized use case, will revisit later
Contact: reuben

## What This Module Does

The Swank module provides a Rust client for the SLIME/Swank protocol, allowing
AI agents to:

- Connect to a running SBCL instance with Swank server
- Evaluate Common Lisp code interactively
- Inspect and debug running Lisp systems
- Perform live code modifications

## Files

- `mod.rs` - Module exports and type definitions
- `client.rs` - Swank protocol client implementation
- `codec.rs` - S-expression parsing and encoding
- `launcher.rs` - SBCL process management
- `registry.rs` - Connection registry for multiple REPL sessions
- `integration_tests.rs` - Test suite

## Usage Notes

This module is fully functional. If you need AI-assisted Common Lisp development:

1. Install SBCL with Quicklisp
2. Load Swank: `(ql:quickload :swank)`
3. Start server: `(swank:create-server :port 4005 :dont-close t)`
4. Use the Descartes Swank client to connect

## Why Frozen

The Lisp AI development use case is specialized. The codebase is focusing on
Claude/Grok-based agents and mainstream development workflows. Swank integration
works but isn't a priority for active development.
