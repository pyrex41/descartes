# JSON Event Format Specification

This document specifies the JSON formats used by SCUD CLI flags for programmatic integration with GUIs and other tools.

## Overview

SCUD provides two JSON output modes:

| Flag | Command | Purpose |
|------|---------|---------|
| `--json` | `waves`, `list` | Complete JSON output |
| `--json-events` | `waves`, `swarm` | Streaming JSON events (one per line) |

## Streaming Events (`--json-events`)

All streaming events use newline-delimited JSON (NDJSON). Each line is a complete JSON object that can be parsed independently.

### Common Event Structure

```json
{
  "event": "<event_type>",
  ...fields specific to event type
}
```

The `event` field uses `snake_case` naming convention.

---

## Swarm Events

When running `scud swarm --json-events`, the following events are emitted:

### `swarm_started`

Emitted when swarm execution begins.

```json
{
  "event": "swarm_started",
  "tag": "feature",
  "total_waves": 3
}
```

| Field | Type | Description |
|-------|------|-------------|
| `tag` | string | SCUD tag being executed |
| `total_waves` | number | Total number of waves to execute |

### `wave_started`

Emitted when a new wave begins execution.

```json
{
  "event": "wave_started",
  "wave": 0,
  "tasks": ["1", "2", "3"]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `wave` | number | Zero-indexed wave number |
| `tasks` | string[] | Task IDs in this wave |

### `task_started`

Emitted when an individual task begins execution.

```json
{
  "event": "task_started",
  "task_id": "1.2"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | string | Unique task identifier |

### `task_output`

Emitted when a task produces output.

```json
{
  "event": "task_output",
  "task_id": "1.2",
  "text": "Building component..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | string | Task producing output |
| `text` | string | Output text (may include ANSI codes) |

### `task_completed`

Emitted when a task finishes execution.

```json
{
  "event": "task_completed",
  "task_id": "1.2",
  "success": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | string | Completed task identifier |
| `success` | boolean | Whether task succeeded |

### `validation_started`

Emitted when backpressure validation begins.

```json
{
  "event": "validation_started"
}
```

No additional fields.

### `validation_completed`

Emitted when backpressure validation finishes.

```json
{
  "event": "validation_completed",
  "passed": false,
  "output": "Build failed: error[E0425]: cannot find value `foo`"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `passed` | boolean | Whether validation passed |
| `output` | string | Validation command output (optional) |

### `wave_completed`

Emitted when all tasks in a wave have finished.

```json
{
  "event": "wave_completed",
  "wave": 0
}
```

| Field | Type | Description |
|-------|------|-------------|
| `wave` | number | Zero-indexed wave number |

### `swarm_completed`

Emitted when swarm execution finishes.

```json
{
  "event": "swarm_completed",
  "success": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `success` | boolean | Whether overall execution succeeded |

---

## Wave Events

When running `scud waves --json-events`, wave information is streamed:

```json
{"type": "wave", "wave_number": 1, "task_ids": ["1", "2"], "task_count": 2, "is_final": false}
{"type": "wave", "wave_number": 2, "task_ids": ["3"], "task_count": 1, "is_final": true}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"wave"` |
| `wave_number` | number | One-indexed wave number |
| `task_ids` | string[] | Task IDs in this wave |
| `task_count` | number | Number of tasks in wave |
| `is_final` | boolean | Whether this is the last wave |

---

## Complete JSON Output (`--json`)

### Waves Output

`scud waves --json` returns:

```json
{
  "waves": [
    {
      "wave_number": 1,
      "task_ids": ["1", "2"],
      "task_count": 2
    },
    {
      "wave_number": 2,
      "task_ids": ["3"],
      "task_count": 1
    }
  ],
  "total_tasks": 3,
  "total_waves": 2
}
```

### Task List Output

`scud list --json` returns an array of tasks:

```json
[
  {
    "id": "1",
    "title": "Implement user model",
    "status": "Pending",
    "dependencies": [],
    "priority": "High",
    "complexity": 2
  },
  {
    "id": "2",
    "title": "Add authentication",
    "status": "Done",
    "dependencies": ["1"],
    "priority": "Medium",
    "complexity": 3
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique task identifier |
| `title` | string | Task title |
| `status` | string | Task status: `Pending`, `InProgress`, `Done`, `Blocked`, `Failed` |
| `dependencies` | string[] | IDs of tasks this depends on |
| `priority` | string? | Priority level (optional) |
| `complexity` | number? | Complexity score (optional) |

---

## GUI Integration

The Descartes GUI (`descartes-gui`) uses these JSON formats via the ScudBridge module:

```
┌─────────────────────────────────────────────────────────┐
│                    Descartes GUI                         │
├─────────────────────────────────────────────────────────┤
│                     ScudBridge                           │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  scud list --json       → TasksLoaded event         │ │
│  │  scud waves --json      → WavesComputed event       │ │
│  │  scud swarm --json-events → Stream of ScudEvents    │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Event Flow Example

```
$ scud swarm --tag feature --json-events

{"event":"swarm_started","tag":"feature","total_waves":2}
{"event":"wave_started","wave":0,"tasks":["1","2"]}
{"event":"task_started","task_id":"1"}
{"event":"task_output","task_id":"1","text":"Creating file..."}
{"event":"task_completed","task_id":"1","success":true}
{"event":"task_started","task_id":"2"}
{"event":"task_completed","task_id":"2","success":true}
{"event":"validation_started"}
{"event":"validation_completed","passed":true,"output":""}
{"event":"wave_completed","wave":0}
{"event":"wave_started","wave":1,"tasks":["3"]}
{"event":"task_started","task_id":"3"}
{"event":"task_completed","task_id":"3","success":true}
{"event":"wave_completed","wave":1}
{"event":"swarm_completed","success":true}
```

### Parsing Example (Rust)

```rust
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ScudEvent {
    SwarmStarted { tag: String, total_waves: usize },
    WaveStarted { wave: usize, tasks: Vec<String> },
    TaskStarted { task_id: String },
    TaskOutput { task_id: String, text: String },
    TaskCompleted { task_id: String, success: bool },
    ValidationStarted,
    ValidationCompleted { passed: bool, output: String },
    WaveCompleted { wave: usize },
    SwarmCompleted { success: bool },
}

// Parse each line
for line in stdout.lines() {
    if let Ok(event) = serde_json::from_str::<ScudEvent>(&line) {
        match event {
            ScudEvent::TaskCompleted { task_id, success } => {
                println!("Task {} finished: {}", task_id, success);
            }
            // handle other events...
        }
    }
}
```

---

## Error Handling

When JSON parsing fails or commands error:
- Non-JSON lines may be emitted (e.g., progress messages)
- Check exit code for overall success
- stderr contains error messages

Consumers should:
1. Try parsing each line as JSON
2. Handle non-JSON lines gracefully (log or display as raw output)
3. Check process exit status for final result
