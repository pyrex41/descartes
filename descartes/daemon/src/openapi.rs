/// OpenAPI 3.0 schema generation for RPC API

use serde_json::{json, Value};

/// Generate OpenAPI schema
pub fn generate_openapi_schema() -> Value {
    json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Descartes RPC Daemon API",
            "description": "JSON-RPC 2.0 API for managing agents, workflows, and system state",
            "version": crate::VERSION,
            "contact": {
                "name": "Descartes Team",
                "url": "https://github.com/descartes"
            },
            "license": {
                "name": "MIT"
            }
        },
        "servers": [
            {
                "url": "http://127.0.0.1:8080",
                "description": "Local development server"
            },
            {
                "url": "http://{host}:{port}",
                "description": "Custom server",
                "variables": {
                    "host": {
                        "default": "localhost",
                        "description": "Server hostname"
                    },
                    "port": {
                        "default": "8080",
                        "description": "Server port"
                    }
                }
            }
        ],
        "paths": {
            "/": {
                "post": {
                    "summary": "JSON-RPC 2.0 endpoint",
                    "description": "Submit JSON-RPC 2.0 requests",
                    "tags": ["RPC"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/JsonRpcRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/JsonRpcResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Bad request"
                        },
                        "500": {
                            "description": "Internal server error"
                        }
                    }
                },
                "get": {
                    "summary": "Server information",
                    "description": "Get information about the RPC server and available methods",
                    "tags": ["System"],
                    "responses": {
                        "200": {
                            "description": "Server information",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "version": { "type": "string" },
                                            "methods": {
                                                "type": "array",
                                                "items": { "type": "string" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/metrics": {
                "get": {
                    "summary": "Prometheus metrics",
                    "description": "Get Prometheus-format metrics",
                    "tags": ["Metrics"],
                    "responses": {
                        "200": {
                            "description": "Metrics in Prometheus format",
                            "content": {
                                "text/plain": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "JsonRpcRequest": {
                    "type": "object",
                    "required": ["jsonrpc", "method"],
                    "properties": {
                        "jsonrpc": {
                            "type": "string",
                            "enum": ["2.0"],
                            "description": "JSON-RPC version"
                        },
                        "method": {
                            "type": "string",
                            "description": "RPC method name",
                            "enum": [
                                "agent.spawn",
                                "agent.list",
                                "agent.kill",
                                "agent.logs",
                                "workflow.execute",
                                "state.query",
                                "system.health",
                                "system.metrics"
                            ]
                        },
                        "params": {
                            "description": "Method parameters"
                        },
                        "id": {
                            "description": "Request ID (string or number)"
                        }
                    }
                },
                "JsonRpcResponse": {
                    "type": "object",
                    "required": ["jsonrpc"],
                    "properties": {
                        "jsonrpc": {
                            "type": "string",
                            "enum": ["2.0"]
                        },
                        "result": {
                            "description": "Response result (if successful)"
                        },
                        "error": {
                            "$ref": "#/components/schemas/JsonRpcError"
                        },
                        "id": {
                            "description": "Request ID"
                        }
                    }
                },
                "JsonRpcError": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {
                            "type": "integer",
                            "description": "Error code",
                            "enum": [-32700, -32600, -32601, -32603, -32001, -32002, -32003, -32004]
                        },
                        "message": {
                            "type": "string",
                            "description": "Error message"
                        },
                        "data": {
                            "description": "Additional error data"
                        }
                    }
                },
                "AgentSpawnRequest": {
                    "type": "object",
                    "required": ["name", "agent_type"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Agent name"
                        },
                        "agent_type": {
                            "type": "string",
                            "description": "Agent type"
                        },
                        "config": {
                            "type": "object",
                            "description": "Agent configuration"
                        }
                    }
                },
                "AgentSpawnResponse": {
                    "type": "object",
                    "properties": {
                        "agent_id": {
                            "type": "string",
                            "description": "Agent ID"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["running", "paused", "stopped", "failed", "terminated"]
                        },
                        "message": {
                            "type": "string"
                        }
                    }
                },
                "AgentListResponse": {
                    "type": "object",
                    "properties": {
                        "agents": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/AgentInfo"
                            }
                        },
                        "count": {
                            "type": "integer"
                        }
                    }
                },
                "AgentInfo": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "name": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["running", "paused", "stopped", "failed", "terminated"]
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "updated_at": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "pid": { "type": "integer" },
                        "config": { "type": "object" }
                    }
                },
                "HealthCheckResponse": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" },
                        "version": { "type": "string" },
                        "uptime_secs": { "type": "integer" },
                        "timestamp": {
                            "type": "string",
                            "format": "date-time"
                        }
                    }
                },
                "MetricsResponse": {
                    "type": "object",
                    "properties": {
                        "agents": {
                            "$ref": "#/components/schemas/MetricsAgents"
                        },
                        "system": {
                            "$ref": "#/components/schemas/MetricsSystem"
                        },
                        "timestamp": {
                            "type": "string",
                            "format": "date-time"
                        }
                    }
                },
                "MetricsAgents": {
                    "type": "object",
                    "properties": {
                        "total": { "type": "integer" },
                        "running": { "type": "integer" },
                        "paused": { "type": "integer" },
                        "stopped": { "type": "integer" },
                        "failed": { "type": "integer" }
                    }
                },
                "MetricsSystem": {
                    "type": "object",
                    "properties": {
                        "uptime_secs": { "type": "integer" },
                        "memory_usage_mb": { "type": "number" },
                        "cpu_usage_percent": { "type": "number" },
                        "active_connections": { "type": "integer" }
                    }
                }
            }
        },
        "tags": [
            {
                "name": "Agent",
                "description": "Agent management operations"
            },
            {
                "name": "Workflow",
                "description": "Workflow execution operations"
            },
            {
                "name": "State",
                "description": "State query and management"
            },
            {
                "name": "System",
                "description": "System and health operations"
            },
            {
                "name": "Metrics",
                "description": "Metrics and monitoring"
            },
            {
                "name": "RPC",
                "description": "JSON-RPC 2.0 endpoint"
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_schema_generation() {
        let schema = generate_openapi_schema();
        assert!(schema["openapi"].as_str().unwrap() == "3.0.0");
        assert!(schema["info"]["title"].as_str().is_some());
        assert!(schema["paths"].is_object());
        assert!(schema["components"]["schemas"].is_object());
    }
}
