# Swarm.toml Parser - Usage Guide

**Version**: 1.0
**Date**: November 23, 2025

---

## Quick Start

### 1. Basic Parsing

```rust
use descartes_core::swarm_parser::SwarmParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let config = parser.parse_file("Swarm.toml")?;
    Ok(())
}
```

### 2. Parse and Validate

```rust
use descartes_core::swarm_parser::SwarmParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate("Swarm.toml")?;

    // Check for unreachable states
    for workflow in &workflows {
        workflow.check_unreachable_states()?;
    }

    Ok(())
}
```

### 3. Generate Code

```rust
use descartes_core::swarm_parser::SwarmParser;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate("Swarm.toml")?;

    for workflow in workflows {
        // Generate state machine code
        let code = workflow.generate_state_machine_code();
        fs::write("generated_workflow.rs", code)?;

        // Generate documentation
        let diagram = workflow.generate_mermaid_diagram();
        fs::write("workflow_diagram.md", diagram)?;
    }

    Ok(())
}
```

---

## Common Tasks

### Task 1: Validate a Workflow File

```rust
use descartes_core::swarm_parser::{SwarmParser, SwarmParseError};

fn validate_workflow(path: &str) -> Result<(), SwarmParseError> {
    let parser = SwarmParser::new();
    let config = parser.parse_file(path)?;
    parser.validate_config(&config)?;

    for workflow in config.workflows {
        let validated = parser.validate_workflow(&workflow, &config)?;
        validated.check_unreachable_states()?;
    }

    println!("Workflow validation successful!");
    Ok(())
}
```

### Task 2: Extract Workflow Information

```rust
use descartes_core::swarm_parser::SwarmParser;

fn analyze_workflow(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path)?;

    for workflow in workflows {
        println!("Workflow: {}", workflow.name);
        println!("  Initial State: {}", workflow.metadata.initial_state);
        println!("  States:");

        for (name, state) in &workflow.states {
            println!("    - {} (reachable: {})", name, state.reachable);
            if !state.handlers.is_empty() {
                for handler in &state.handlers {
                    println!("      - Event: {} -> {}", handler.event, handler.target);
                }
            }
        }

        println!("  Agents: {:?}", workflow.agents.keys().collect::<Vec<_>>());
        println!("  Resources: {:?}", workflow.resources.keys().collect::<Vec<_>>());
    }

    Ok(())
}
```

### Task 3: Generate All Code Artifacts

```rust
use descartes_core::swarm_parser::SwarmParser;
use std::fs;
use std::path::Path;

fn generate_all(workflow_path: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(workflow_path)?;

    for workflow in workflows {
        let safe_name = workflow.name.to_lowercase().replace(" ", "_");

        // Generate state machine
        let state_code = workflow.generate_state_enum();
        fs::write(
            Path::new(output_dir).join(format!("{}_states.rs", safe_name)),
            state_code,
        )?;

        // Generate events
        let event_code = workflow.generate_event_enum();
        fs::write(
            Path::new(output_dir).join(format!("{}_events.rs", safe_name)),
            event_code,
        )?;

        // Generate context
        let context_code = workflow.generate_context_struct();
        fs::write(
            Path::new(output_dir).join(format!("{}_context.rs", safe_name)),
            context_code,
        )?;

        // Generate state machine
        let machine_code = workflow.generate_state_machine_code();
        fs::write(
            Path::new(output_dir).join(format!("{}_machine.rs", safe_name)),
            machine_code,
        )?;

        // Generate documentation
        let diagram = workflow.generate_mermaid_diagram();
        fs::write(
            Path::new(output_dir).join(format!("{}_diagram.md", safe_name)),
            diagram,
        )?;
    }

    Ok(())
}
```

### Task 4: Find Unreachable States

```rust
use descartes_core::swarm_parser::SwarmParser;

fn find_unreachable(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path)?;

    for workflow in workflows {
        let unreachable: Vec<_> = workflow
            .states
            .iter()
            .filter(|(_, state)| !state.reachable)
            .map(|(name, _)| name)
            .collect();

        if !unreachable.is_empty() {
            println!("Warning: Unreachable states in {}: {:?}", workflow.name, unreachable);
        }
    }

    Ok(())
}
```

### Task 5: Find Workflows with Specific Features

```rust
use descartes_core::swarm_parser::SwarmParser;

fn find_parallel_workflows(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path)?;

    for workflow in workflows {
        let parallel_states: Vec<_> = workflow
            .states
            .iter()
            .filter(|(_, state)| state.parallel_execution)
            .map(|(name, _)| name)
            .collect();

        if !parallel_states.is_empty() {
            println!(
                "Workflow {} has parallel states: {:?}",
                workflow.name, parallel_states
            );
        }
    }

    Ok(())
}
```

---

## Error Handling

### Understanding Error Types

```rust
use descartes_core::swarm_parser::{SwarmParser, SwarmParseError};

fn handle_errors(path: &str) {
    let parser = SwarmParser::new();

    match parser.parse_file(path) {
        Ok(config) => println!("Parsed successfully"),
        Err(SwarmParseError::TomlError(e)) => {
            eprintln!("TOML parse error: {}", e);
        }
        Err(SwarmParseError::IoError(e)) => {
            eprintln!("File I/O error: {}", e);
        }
        Err(SwarmParseError::ValidationError(msg)) => {
            eprintln!("Validation error: {}", msg);
        }
        Err(SwarmParseError::UnreachableState(msg)) => {
            eprintln!("Unreachable state: {}", msg);
        }
        Err(SwarmParseError::CyclicDependency(msg)) => {
            eprintln!("Cyclic dependency detected: {}", msg);
        }
        Err(SwarmParseError::InvalidAgent(msg)) => {
            eprintln!("Invalid agent reference: {}", msg);
        }
        Err(SwarmParseError::InvalidResource(msg)) => {
            eprintln!("Invalid resource reference: {}", msg);
        }
        Err(SwarmParseError::InvalidGuard(msg)) => {
            eprintln!("Invalid guard reference: {}", msg);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Detailed Error Information

```rust
use descartes_core::swarm_parser::SwarmParser;

fn validate_with_details(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();

    let config = parser.parse_file(path)?;

    if let Err(e) = parser.validate_config(&config) {
        eprintln!("Configuration validation failed:");
        eprintln!("  Error: {}", e);
        return Err(e.into());
    }

    for workflow in &config.workflows {
        match parser.validate_workflow(workflow, &config) {
            Ok(_) => {
                println!("Workflow '{}' validated successfully", workflow.name);
            }
            Err(e) => {
                eprintln!("Workflow '{}' validation failed: {}", workflow.name, e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
```

---

## Integration Examples

### Integration with State Machine Execution

```rust
use descartes_core::swarm_parser::SwarmParser;

fn setup_workflow_execution(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path)?;

    for workflow in workflows {
        // Generate code
        let state_machine_code = workflow.generate_state_machine_code();

        // Could be compiled or executed dynamically
        println!("Generated state machine:\n{}", state_machine_code);

        // Extract workflow metadata
        let initial_state = &workflow.metadata.initial_state;
        let agents = &workflow.agents;

        println!("Initial state: {}", initial_state);
        println!("Available agents: {:?}", agents.keys().collect::<Vec<_>>());
    }

    Ok(())
}
```

### Integration with Agent System

```rust
use descartes_core::swarm_parser::SwarmParser;

fn resolve_agents(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path)?;

    for workflow in workflows {
        for (agent_name, agent_config) in &workflow.agents {
            println!("Agent: {}", agent_name);
            println!("  Model: {}", agent_config.model);

            if let Some(tokens) = agent_config.max_tokens {
                println!("  Max tokens: {}", tokens);
            }

            if let Some(temp) = agent_config.temperature {
                println!("  Temperature: {}", temp);
            }

            if !agent_config.tags.is_empty() {
                println!("  Tags: {:?}", agent_config.tags);
            }
        }
    }

    Ok(())
}
```

### Integration with Resource Management

```rust
use descartes_core::swarm_parser::{SwarmParser, ResourceConfig};

fn setup_resources(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path)?;

    for workflow in workflows {
        for (resource_name, resource_config) in &workflow.resources {
            match resource_config {
                ResourceConfig::Http { endpoint, auth_required, secret_key } => {
                    println!("HTTP Resource: {}", resource_name);
                    println!("  Endpoint: {}", endpoint);
                    println!("  Auth required: {:?}", auth_required);
                    if let Some(key) = secret_key {
                        println!("  Secret key: {}", key);
                    }
                }
                ResourceConfig::Webhook { endpoint, description } => {
                    println!("Webhook Resource: {}", resource_name);
                    println!("  Endpoint: {}", endpoint);
                    if let Some(desc) = description {
                        println!("  Description: {}", desc);
                    }
                }
                ResourceConfig::Database { connection_string, pool_size } => {
                    println!("Database Resource: {}", resource_name);
                    if let Some(size) = pool_size {
                        println!("  Pool size: {}", size);
                    }
                }
                ResourceConfig::Custom { config } => {
                    println!("Custom Resource: {}", resource_name);
                    println!("  Config: {}", config);
                }
            }
        }
    }

    Ok(())
}
```

---

## Best Practices

### 1. Always Validate After Parsing

```rust
// Good
let parser = SwarmParser::new();
let workflows = parser.parse_and_validate("Swarm.toml")?;

// Less ideal
let config = parser.parse_file("Swarm.toml")?;
// Missing validation
```

### 2. Check for Unreachable States

```rust
// Good
for workflow in &workflows {
    workflow.check_unreachable_states()?;
}

// Less ideal - may silently ignore unreachable states
```

### 3. Handle All Error Cases

```rust
// Good
match parser.parse_file(path) {
    Ok(config) => { /* ... */ },
    Err(e) => eprintln!("Failed to parse: {}", e),
}

// Less ideal
let config = parser.parse_file(path).unwrap(); // Panics on error
```

### 4. Separate Parsing and Validation

```rust
// For complex scenarios
let parser = SwarmParser::new();
let config = parser.parse_file("Swarm.toml")?;

// Validate individual workflows
for workflow in config.workflows {
    match parser.validate_workflow(&workflow, &config) {
        Ok(validated) => { /* use validated */ },
        Err(e) => { /* handle error */ },
    }
}
```

---

## Performance Tips

### 1. Parse Once, Reuse Many Times

```rust
fn bad_approach(path: &str, count: usize) {
    let parser = SwarmParser::new();
    for _ in 0..count {
        let workflows = parser.parse_and_validate(path).unwrap();
        // ...
    }
}

fn good_approach(path: &str, count: usize) {
    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(path).unwrap();
    for _ in 0..count {
        // Reuse parsed workflows
        for w in &workflows {
            // ...
        }
    }
}
```

### 2. Cache Generated Code

```rust
use std::fs;
use std::path::Path;

fn generate_with_caching(
    workflow_path: &str,
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if already generated
    let cache_file = format!("{}/generated.rs", output_dir);
    if Path::new(&cache_file).exists() {
        return Ok(());
    }

    let parser = SwarmParser::new();
    let workflows = parser.parse_and_validate(workflow_path)?;

    for workflow in workflows {
        let code = workflow.generate_state_machine_code();
        fs::write(&cache_file, code)?;
    }

    Ok(())
}
```

---

## Debugging Tips

### 1. Print Parsed Structure

```rust
use descartes_core::swarm_parser::SwarmParser;

fn debug_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let config = parser.parse_file(path)?;

    println!("Metadata: {:?}", config.metadata);
    println!("Agents: {:?}", config.agents.keys().collect::<Vec<_>>());
    println!("Resources: {:?}", config.resources.keys().collect::<Vec<_>>());
    println!("Workflows: {:?}", config.workflows.iter().map(|w| &w.name).collect::<Vec<_>>());

    Ok(())
}
```

### 2. Trace Validation

```rust
fn trace_validation(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parser = SwarmParser::new();
    let config = parser.parse_file(path)?;

    println!("Step 1: Validate config");
    parser.validate_config(&config)?;
    println!("  ✓ Config validation passed");

    for workflow in &config.workflows {
        println!("Step 2: Validate workflow '{}'", workflow.name);
        match parser.validate_workflow(workflow, &config) {
            Ok(validated) => {
                println!("  ✓ Workflow validation passed");
                println!("  States: {}", validated.states.len());

                let reachable_count = validated.states.iter().filter(|(_, s)| s.reachable).count();
                println!("  Reachable states: {}", reachable_count);
            }
            Err(e) => {
                println!("  ✗ Validation failed: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
```

---

## References

- [Swarm.toml Schema](./SWARM_TOML_SCHEMA.md)
- [Parser Implementation](./SWARM_PARSER_IMPLEMENTATION.md)
- [API Documentation](https://docs.rs/descartes-core/latest/descartes_core/swarm_parser/)

---

## FAQ

**Q: Can I have multiple workflows in one Swarm.toml file?**
A: Yes! Use multiple `[[workflows]]` sections. Each will be parsed and validated independently.

**Q: What happens if I have unreachable states?**
A: The parser will allow them but mark them as unreachable. Call `check_unreachable_states()` to get an error if any exist.

**Q: Can I use environment variables in configuration?**
A: Currently the parser supports literal strings. Environment variable interpolation is planned for v1.1.

**Q: What's the maximum number of states?**
A: There's no hard limit, but performance depends on your hardware. The algorithm is O(states + edges).

**Q: Can I generate TypeScript or Python code?**
A: Currently, code generation targets Rust. Generating other languages is planned for v1.1.

**Q: How do I handle circular workflows?**
A: Circular transitions (state A -> B -> A) are allowed if they form a valid DAG from the initial state. True cycles are detected and rejected.
