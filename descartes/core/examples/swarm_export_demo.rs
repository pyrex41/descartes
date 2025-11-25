/// Example: DAG to Swarm.toml Export and Import
///
/// This example demonstrates how to:
/// 1. Create a visual DAG representation
/// 2. Export it to Swarm.toml format
/// 3. Import it back to DAG
/// 4. Save/load from files
///
/// Run with: cargo run --example swarm_export_demo
use descartes_core::dag::{DAGEdge, DAGNode, EdgeType, DAG};
use descartes_core::dag_swarm_export::{
    export_dag_to_swarm_toml, import_swarm_toml_to_dag, load_dag_from_swarm_toml,
    save_dag_as_swarm_toml, SwarmExportConfig,
};
use descartes_core::swarm_parser::{AgentConfig, ResourceConfig, SwarmConfig};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DAG to Swarm.toml Export Demo ===\n");

    // Example 1: Simple approval workflow
    example_simple_approval()?;

    // Example 2: Parallel code review workflow
    example_parallel_review()?;

    // Example 3: Complex development workflow
    example_development_workflow()?;

    // Example 4: Roundtrip conversion
    example_roundtrip()?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

/// Example 1: Simple approval workflow
fn example_simple_approval() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 1: Simple Approval Workflow ---");

    let mut dag = DAG::new("Simple Approval");
    dag.description = Some("A basic request approval workflow".to_string());

    // Create states
    let pending = DAGNode::new_auto("Pending")
        .with_description("Request awaiting approval")
        .with_metadata("agents", serde_json::json!(["approver"]))
        .with_metadata("entry_actions", serde_json::json!(["log_submission"]))
        .with_position(100.0, 200.0);

    let approved = DAGNode::new_auto("Approved")
        .with_description("Request approved and processed")
        .with_metadata(
            "entry_actions",
            serde_json::json!(["process_request", "notify_requester"]),
        )
        .with_position(300.0, 100.0);

    let rejected = DAGNode::new_auto("Rejected")
        .with_description("Request rejected")
        .with_metadata("entry_actions", serde_json::json!(["notify_requester"]))
        .with_position(300.0, 300.0);

    let pending_id = pending.node_id;
    let approved_id = approved.node_id;
    let rejected_id = rejected.node_id;

    dag.add_node(pending)?;
    dag.add_node(approved)?;
    dag.add_node(rejected)?;

    // Add edges with event names
    let mut approve_edge = DAGEdge::dependency(pending_id, approved_id);
    approve_edge.label = Some("approve".to_string());
    dag.add_edge(approve_edge)?;

    let mut reject_edge = DAGEdge::dependency(pending_id, rejected_id);
    reject_edge.label = Some("reject".to_string());
    dag.add_edge(reject_edge)?;

    // Configure export
    let config = SwarmExportConfig::default()
        .with_workflow_name("simple_approval")
        .with_description("Simple approval workflow for requests")
        .with_agent_config(
            "approver",
            AgentConfig {
                model: "claude-3-haiku".to_string(),
                max_tokens: Some(1000),
                temperature: Some(0.5),
                tags: vec!["review".to_string(), "approval".to_string()],
            },
        )
        .with_timeout(1800)
        .with_retries(1, 30);

    // Export to Swarm.toml
    let swarm_toml = export_dag_to_swarm_toml(&dag, &config)?;

    println!("Generated Swarm.toml:");
    println!("{}\n", swarm_toml);

    // Validate statistics
    let stats = dag.statistics()?;
    println!("DAG Statistics:");
    println!("  Nodes: {}", stats.node_count);
    println!("  Edges: {}", stats.edge_count);
    println!("  Start nodes: {}", stats.start_nodes);
    println!("  End nodes: {}", stats.end_nodes);
    println!("  Is acyclic: {}", stats.is_acyclic);
    println!();

    Ok(())
}

/// Example 2: Parallel code review workflow
fn example_parallel_review() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 2: Parallel Code Review Workflow ---");

    let mut dag = DAG::new("Parallel Code Review");
    dag.description = Some("Multiple agents review code simultaneously".to_string());

    // Create states
    let submitted = DAGNode::new_auto("SubmittedForReview")
        .with_description("Code submitted, initiating parallel reviews")
        .with_metadata(
            "entry_actions",
            serde_json::json!(["prepare_code_for_review", "assign_reviewers"]),
        )
        .with_position(100.0, 200.0);

    let reviews_gathered = DAGNode::new_auto("ReviewsGathered")
        .with_description("All parallel reviews collected and analyzed")
        .with_metadata(
            "agents",
            serde_json::json!(["architect", "security", "performance", "maintainability"]),
        )
        .with_metadata(
            "entry_actions",
            serde_json::json!(["aggregate_feedback", "analyze_consensus"]),
        )
        .with_metadata("parallel_execution", true)
        .with_position(300.0, 200.0);

    let approved = DAGNode::new_auto("Approved")
        .with_description("Code approved by parallel review consensus")
        .with_metadata(
            "entry_actions",
            serde_json::json!(["merge_code", "notify_team"]),
        )
        .with_position(500.0, 100.0);

    let changes_needed = DAGNode::new_auto("ChangesNeeded")
        .with_description("Changes needed based on parallel reviews")
        .with_metadata(
            "entry_actions",
            serde_json::json!(["compile_feedback", "notify_developer"]),
        )
        .with_position(500.0, 300.0);

    let submitted_id = submitted.node_id;
    let reviews_id = reviews_gathered.node_id;
    let approved_id = approved.node_id;
    let changes_id = changes_needed.node_id;

    dag.add_node(submitted)?;
    dag.add_node(reviews_gathered)?;
    dag.add_node(approved)?;
    dag.add_node(changes_needed)?;

    // Add edges
    let mut reviews_complete = DAGEdge::dependency(submitted_id, reviews_id);
    reviews_complete.label = Some("reviews_complete".to_string());
    dag.add_edge(reviews_complete)?;

    let mut consensus_approved = DAGEdge::dependency(reviews_id, approved_id);
    consensus_approved.label = Some("consensus_approved".to_string());
    consensus_approved.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["consensus_approved"]),
    );
    dag.add_edge(consensus_approved)?;

    let mut issues_found = DAGEdge::dependency(reviews_id, changes_id);
    issues_found.label = Some("issues_found".to_string());
    issues_found
        .metadata
        .insert("guards".to_string(), serde_json::json!(["any_critical"]));
    dag.add_edge(issues_found)?;

    // Configure export with multiple agents
    let mut config = SwarmExportConfig::default()
        .with_workflow_name("parallel_code_review")
        .with_description("Multiple agents review code simultaneously")
        .with_timeout(1800)
        .with_retries(2, 60);

    // Add multiple specialized agents
    config = config
        .with_agent_config(
            "architect",
            AgentConfig {
                model: "claude-3-opus".to_string(),
                max_tokens: Some(4000),
                temperature: Some(0.7),
                tags: vec!["design".to_string(), "architecture".to_string()],
            },
        )
        .with_agent_config(
            "security",
            AgentConfig {
                model: "claude-3-opus".to_string(),
                max_tokens: Some(3000),
                temperature: Some(0.5),
                tags: vec!["security".to_string(), "vulnerability".to_string()],
            },
        )
        .with_agent_config(
            "performance",
            AgentConfig {
                model: "claude-3-sonnet".to_string(),
                max_tokens: Some(2000),
                temperature: Some(0.5),
                tags: vec!["performance".to_string(), "optimization".to_string()],
            },
        )
        .with_agent_config(
            "maintainability",
            AgentConfig {
                model: "claude-3-sonnet".to_string(),
                max_tokens: Some(2000),
                temperature: Some(0.5),
                tags: vec!["maintainability".to_string(), "code_quality".to_string()],
            },
        );

    // Add guards
    config = config
        .with_guard(
            "consensus_approved",
            "context.approval_votes >= 3".to_string(),
        )
        .with_guard("any_critical", "context.critical_issues > 0".to_string());

    let swarm_toml = export_dag_to_swarm_toml(&dag, &config)?;

    println!("Generated Swarm.toml (first 1000 chars):");
    println!("{}\n", &swarm_toml[..swarm_toml.len().min(1000)]);

    Ok(())
}

/// Example 3: Complex development workflow with hierarchy
fn example_development_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 3: Development Workflow with Hierarchical States ---");

    let mut dag = DAG::new("Development Workflow");
    dag.description = Some("Multi-phase development process".to_string());

    // Create parent state metadata marker
    let in_progress = DAGNode::new_auto("InProgress")
        .with_description("Development work in progress (parent state)")
        .with_metadata(
            "entry_actions",
            serde_json::json!(["log_start", "allocate_resources"]),
        )
        .with_metadata(
            "exit_actions",
            serde_json::json!(["log_end", "release_resources"]),
        )
        .with_position(300.0, 100.0);

    // Phase states (children of InProgress)
    let planning = DAGNode::new_auto("Planning")
        .with_description("Planning phase - requirements and design")
        .with_metadata("parent", "InProgress")
        .with_metadata("agents", serde_json::json!(["architect"]))
        .with_metadata(
            "entry_actions",
            serde_json::json!(["initialize_sprint", "gather_requirements"]),
        )
        .with_metadata("exit_actions", serde_json::json!(["review_plan"]))
        .with_position(100.0, 200.0);

    let implementation = DAGNode::new_auto("Implementation")
        .with_description("Implementation phase - coding and development")
        .with_metadata("parent", "InProgress")
        .with_metadata("agents", serde_json::json!(["developer"]))
        .with_metadata(
            "entry_actions",
            serde_json::json!(["create_branch", "setup_environment"]),
        )
        .with_metadata("exit_actions", serde_json::json!(["commit_changes"]))
        .with_position(200.0, 200.0);

    let testing = DAGNode::new_auto("Testing")
        .with_description("Testing phase - QA and validation")
        .with_metadata("parent", "InProgress")
        .with_metadata("agents", serde_json::json!(["tester"]))
        .with_metadata(
            "entry_actions",
            serde_json::json!(["run_tests", "generate_report"]),
        )
        .with_metadata("exit_actions", serde_json::json!(["archive_results"]))
        .with_position(300.0, 200.0);

    let deployment = DAGNode::new_auto("Deployment")
        .with_description("Deployment phase - release to production")
        .with_metadata("parent", "InProgress")
        .with_metadata("agents", serde_json::json!(["devops"]))
        .with_metadata(
            "entry_actions",
            serde_json::json!(["prepare_deployment", "notify_stakeholders"]),
        )
        .with_metadata(
            "required_resources",
            serde_json::json!(["deployment_service", "monitoring"]),
        )
        .with_position(400.0, 200.0);

    let complete = DAGNode::new_auto("Complete")
        .with_description("Development workflow completed successfully")
        .with_metadata(
            "entry_actions",
            serde_json::json!(["close_sprint", "generate_summary"]),
        )
        .with_position(500.0, 200.0);

    let blocked = DAGNode::new_auto("Blocked")
        .with_description("Development blocked on dependencies")
        .with_metadata("entry_actions", serde_json::json!(["log_blocker"]))
        .with_position(300.0, 350.0);

    // Add all nodes
    let planning_id = planning.node_id;
    let implementation_id = implementation.node_id;
    let testing_id = testing.node_id;
    let deployment_id = deployment.node_id;
    let complete_id = complete.node_id;
    let blocked_id = blocked.node_id;

    dag.add_node(in_progress)?;
    dag.add_node(planning)?;
    dag.add_node(implementation)?;
    dag.add_node(testing)?;
    dag.add_node(deployment)?;
    dag.add_node(complete)?;
    dag.add_node(blocked)?;

    // Build workflow
    let mut edge = DAGEdge::dependency(planning_id, implementation_id);
    edge.label = Some("plan_approved".to_string());
    dag.add_edge(edge)?;

    let mut edge = DAGEdge::dependency(implementation_id, testing_id);
    edge.label = Some("implementation_complete".to_string());
    dag.add_edge(edge)?;

    let mut edge = DAGEdge::dependency(testing_id, deployment_id);
    edge.label = Some("tests_passed".to_string());
    dag.add_edge(edge)?;

    let mut edge = DAGEdge::dependency(deployment_id, complete_id);
    edge.label = Some("deployment_successful".to_string());
    edge.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["no_blocking_issues"]),
    );
    dag.add_edge(edge)?;

    // Add blocked transitions
    let mut edge = DAGEdge::dependency(planning_id, blocked_id);
    edge.label = Some("blocked".to_string());
    dag.add_edge(edge)?;

    let mut edge = DAGEdge::dependency(implementation_id, blocked_id);
    edge.label = Some("blocked".to_string());
    dag.add_edge(edge)?;

    // Configure export
    let mut config = SwarmExportConfig::default()
        .with_workflow_name("development_workflow")
        .with_description("Hierarchical development process with multiple phases")
        .with_initial_state("Planning")
        .with_timeout(86400)
        .with_retries(2, 300);

    // Add agents
    config = config
        .with_agent_config(
            "architect",
            AgentConfig {
                model: "claude-3-opus".to_string(),
                max_tokens: Some(6000),
                temperature: Some(0.7),
                tags: vec!["planning".to_string(), "design".to_string()],
            },
        )
        .with_agent_config(
            "developer",
            AgentConfig {
                model: "claude-3-sonnet".to_string(),
                max_tokens: Some(4000),
                temperature: Some(0.5),
                tags: vec!["implementation".to_string()],
            },
        )
        .with_agent_config(
            "tester",
            AgentConfig {
                model: "claude-3-haiku".to_string(),
                max_tokens: Some(2000),
                temperature: Some(0.2),
                tags: vec!["testing".to_string(), "qa".to_string()],
            },
        )
        .with_agent_config(
            "devops",
            AgentConfig {
                model: "claude-3-sonnet".to_string(),
                max_tokens: Some(3000),
                temperature: Some(0.5),
                tags: vec!["deployment".to_string(), "infrastructure".to_string()],
            },
        );

    // Add resources
    config = config
        .with_resource(
            "deployment_service",
            ResourceConfig::Http {
                endpoint: "https://deployment.internal".to_string(),
                auth_required: Some(true),
                secret_key: Some("DEPLOYMENT_API_KEY".to_string()),
            },
        )
        .with_resource(
            "monitoring",
            ResourceConfig::Http {
                endpoint: "https://monitoring.internal".to_string(),
                auth_required: Some(true),
                secret_key: Some("MONITORING_API_KEY".to_string()),
            },
        );

    // Add guards
    config = config.with_guard(
        "no_blocking_issues",
        "context.blocking_issues == 0".to_string(),
    );

    let swarm_toml = export_dag_to_swarm_toml(&dag, &config)?;

    println!("Generated Swarm.toml (showing structure):");
    println!("  - {} nodes", dag.nodes.len());
    println!("  - {} edges", dag.edges.len());
    println!("  - {} agents configured", config.agents.len());
    println!("  - {} resources configured", config.resources.len());
    println!();

    // Save to file
    let output_path = PathBuf::from("development_workflow.toml");
    save_dag_as_swarm_toml(&dag, &output_path, &config)?;
    println!("  Saved to: {}", output_path.display());
    println!();

    Ok(())
}

/// Example 4: Roundtrip conversion
fn example_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 4: Roundtrip Conversion Test ---");

    // Create original DAG
    let mut dag1 = DAG::new("Roundtrip Test");
    dag1.description = Some("Testing conversion both ways".to_string());

    let node_a = DAGNode::new_auto("StateA")
        .with_description("First state")
        .with_metadata("agents", serde_json::json!(["agent1"]))
        .with_position(100.0, 100.0);

    let node_b = DAGNode::new_auto("StateB")
        .with_description("Second state")
        .with_metadata("agents", serde_json::json!(["agent2"]))
        .with_position(300.0, 100.0);

    let node_c = DAGNode::new_auto("StateC")
        .with_description("Third state")
        .with_position(500.0, 100.0);

    let id_a = node_a.node_id;
    let id_b = node_b.node_id;
    let id_c = node_c.node_id;

    dag1.add_node(node_a)?;
    dag1.add_node(node_b)?;
    dag1.add_node(node_c)?;

    let mut edge = DAGEdge::dependency(id_a, id_b);
    edge.label = Some("go_to_b".to_string());
    dag1.add_edge(edge)?;

    let mut edge = DAGEdge::dependency(id_b, id_c);
    edge.label = Some("go_to_c".to_string());
    dag1.add_edge(edge)?;

    let config = SwarmExportConfig::default()
        .with_workflow_name("roundtrip_test")
        .with_agent("agent1", "claude-3-opus")
        .with_agent("agent2", "claude-3-sonnet");

    println!("Original DAG:");
    println!("  Nodes: {}", dag1.nodes.len());
    println!("  Edges: {}", dag1.edges.len());

    // Export to Swarm.toml
    let swarm_toml = export_dag_to_swarm_toml(&dag1, &config)?;
    println!("\nExported to Swarm.toml ({} bytes)", swarm_toml.len());

    // Parse back to SwarmConfig
    let swarm_config: SwarmConfig = toml::from_str(&swarm_toml)?;
    println!("Parsed SwarmConfig:");
    println!("  Workflows: {}", swarm_config.workflows.len());

    // Import back to DAG
    let dag2 = import_swarm_toml_to_dag(&swarm_config, 0)?;
    println!("\nImported DAG:");
    println!("  Nodes: {}", dag2.nodes.len());
    println!("  Edges: {}", dag2.edges.len());

    // Verify
    assert_eq!(dag1.nodes.len(), dag2.nodes.len(), "Node count mismatch");
    assert_eq!(dag1.edges.len(), dag2.edges.len(), "Edge count mismatch");

    println!("\n✓ Roundtrip conversion successful!");
    println!();

    Ok(())
}
