/// Example 4: Hierarchical Workflow
///
/// This example demonstrates hierarchical state composition with:
/// - Parent-child state relationships
/// - Sub-workflows
/// - Hierarchical organization
/// - State nesting
///
/// Use case: Large-scale application deployment workflow

use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType};
use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = DAG::new("Application Deployment Workflow");
    dag.description = Some("Hierarchical deployment workflow with sub-processes".to_string());

    // Top-level states
    let init = DAGNode::new_auto("Initialize")
        .with_description("Initialize deployment environment")
        .with_position(400.0, 100.0)
        .with_metadata("agents", serde_json::json!(["deployment_orchestrator"]))
        .with_metadata("entry_actions", serde_json::json!(["check_prerequisites", "allocate_resources"]))
        .with_tag("deployment")
        .with_tag("initialization");

    // Build phase (composite state)
    let build = DAGNode::new_auto("Build")
        .with_description("Build application artifacts")
        .with_position(400.0, 300.0)
        .with_metadata("agents", serde_json::json!(["build_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["prepare_build_env"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_tag("deployment")
        .with_tag("build");

    // Build sub-states
    let compile = DAGNode::new_auto("Compile")
        .with_description("Compile source code")
        .with_position(200.0, 450.0)
        .with_metadata("agents", serde_json::json!(["compiler_agent"]))
        .with_metadata("parent", serde_json::json!("Build"))
        .with_metadata("timeout_seconds", serde_json::json!(600))
        .with_tag("build")
        .with_tag("sub_state");

    let run_tests = DAGNode::new_auto("RunTests")
        .with_description("Run automated tests")
        .with_position(400.0, 450.0)
        .with_metadata("agents", serde_json::json!(["test_runner_agent"]))
        .with_metadata("parent", serde_json::json!("Build"))
        .with_metadata("timeout_seconds", serde_json::json!(900))
        .with_tag("build")
        .with_tag("sub_state");

    let package = DAGNode::new_auto("Package")
        .with_description("Package application")
        .with_position(600.0, 450.0)
        .with_metadata("agents", serde_json::json!(["packaging_agent"]))
        .with_metadata("parent", serde_json::json!("Build"))
        .with_tag("build")
        .with_tag("sub_state");

    // Deploy phase (composite state)
    let deploy = DAGNode::new_auto("Deploy")
        .with_description("Deploy application to environments")
        .with_position(400.0, 700.0)
        .with_metadata("agents", serde_json::json!(["deployment_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["backup_current", "prepare_deployment"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_tag("deployment")
        .with_tag("deploy");

    // Deploy sub-states (parallel deployment to multiple environments)
    let deploy_staging = DAGNode::new_auto("DeployStaging")
        .with_description("Deploy to staging environment")
        .with_position(200.0, 850.0)
        .with_metadata("agents", serde_json::json!(["environment_deployer"]))
        .with_metadata("parent", serde_json::json!("Deploy"))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["staging_cluster"]))
        .with_tag("deploy")
        .with_tag("sub_state");

    let deploy_canary = DAGNode::new_auto("DeployCanary")
        .with_description("Deploy to canary environment")
        .with_position(400.0, 850.0)
        .with_metadata("agents", serde_json::json!(["environment_deployer"]))
        .with_metadata("parent", serde_json::json!("Deploy"))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["canary_cluster"]))
        .with_tag("deploy")
        .with_tag("sub_state");

    let deploy_production = DAGNode::new_auto("DeployProduction")
        .with_description("Deploy to production environment")
        .with_position(600.0, 850.0)
        .with_metadata("agents", serde_json::json!(["environment_deployer"]))
        .with_metadata("parent", serde_json::json!("Deploy"))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["production_cluster"]))
        .with_metadata("timeout_seconds", serde_json::json!(1800))
        .with_tag("deploy")
        .with_tag("sub_state")
        .with_tag("critical");

    // Verification phase
    let verify = DAGNode::new_auto("Verify")
        .with_description("Verify deployment health")
        .with_position(400.0, 1100.0)
        .with_metadata("agents", serde_json::json!(["verification_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["health_check", "smoke_tests", "integration_tests"]))
        .with_metadata("timeout_seconds", serde_json::json!(600))
        .with_tag("deployment")
        .with_tag("verification");

    // Finalize
    let finalize = DAGNode::new_auto("Finalize")
        .with_description("Finalize deployment and cleanup")
        .with_position(400.0, 1300.0)
        .with_metadata("agents", serde_json::json!(["deployment_orchestrator"]))
        .with_metadata("entry_actions", serde_json::json!(["generate_report", "cleanup_temp"]))
        .with_metadata("exit_actions", serde_json::json!(["notify_team", "update_status"]))
        .with_tag("deployment")
        .with_tag("finalization");

    // Rollback state (conditional)
    let rollback = DAGNode::new_auto("Rollback")
        .with_description("Rollback deployment on failure")
        .with_position(700.0, 1100.0)
        .with_metadata("agents", serde_json::json!(["rollback_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["stop_deployment", "restore_backup"]))
        .with_metadata("exit_actions", serde_json::json!(["notify_failure", "log_incident"]))
        .with_tag("deployment")
        .with_tag("error_handling");

    // Store IDs
    let ids = HashMap::from([
        ("init", init.node_id),
        ("build", build.node_id),
        ("compile", compile.node_id),
        ("test", run_tests.node_id),
        ("package", package.node_id),
        ("deploy", deploy.node_id),
        ("deploy_staging", deploy_staging.node_id),
        ("deploy_canary", deploy_canary.node_id),
        ("deploy_prod", deploy_production.node_id),
        ("verify", verify.node_id),
        ("finalize", finalize.node_id),
        ("rollback", rollback.node_id),
    ]);

    // Add all nodes
    dag.add_node(init)?;
    dag.add_node(build)?;
    dag.add_node(compile)?;
    dag.add_node(run_tests)?;
    dag.add_node(package)?;
    dag.add_node(deploy)?;
    dag.add_node(deploy_staging)?;
    dag.add_node(deploy_canary)?;
    dag.add_node(deploy_production)?;
    dag.add_node(verify)?;
    dag.add_node(finalize)?;
    dag.add_node(rollback)?;

    // Main workflow edges
    dag.add_edge(DAGEdge::dependency(*ids.get("init").unwrap(), *ids.get("build").unwrap())
        .with_label("environment_ready"))?;

    dag.add_edge(DAGEdge::dependency(*ids.get("build").unwrap(), *ids.get("deploy").unwrap())
        .with_label("build_complete"))?;

    dag.add_edge(DAGEdge::dependency(*ids.get("deploy").unwrap(), *ids.get("verify").unwrap())
        .with_label("deployment_complete"))?;

    // Build sub-workflow
    dag.add_edge(DAGEdge::dependency(*ids.get("build").unwrap(), *ids.get("compile").unwrap()))?;
    dag.add_edge(DAGEdge::dependency(*ids.get("compile").unwrap(), *ids.get("test").unwrap()))?;
    dag.add_edge(DAGEdge::dependency(*ids.get("test").unwrap(), *ids.get("package").unwrap()))?;

    // Deploy sub-workflow (parallel)
    dag.add_edge(DAGEdge::dependency(*ids.get("deploy").unwrap(), *ids.get("deploy_staging").unwrap()))?;
    dag.add_edge(DAGEdge::dependency(*ids.get("deploy").unwrap(), *ids.get("deploy_canary").unwrap()))?;
    dag.add_edge(DAGEdge::dependency(*ids.get("deploy_staging").unwrap(), *ids.get("deploy_prod").unwrap())
        .with_label("staging_validated"))?;
    dag.add_edge(DAGEdge::dependency(*ids.get("deploy_canary").unwrap(), *ids.get("deploy_prod").unwrap())
        .with_label("canary_validated"))?;

    // Verification with conditional paths
    let mut verify_success = DAGEdge::dependency(*ids.get("verify").unwrap(), *ids.get("finalize").unwrap())
        .with_label("verification_passed");
    verify_success.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["health_check_passed"])
    );
    dag.add_edge(verify_success)?;

    let mut verify_failed = DAGEdge::dependency(*ids.get("verify").unwrap(), *ids.get("rollback").unwrap())
        .with_label("verification_failed");
    verify_failed.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["health_check_failed"])
    );
    dag.add_edge(verify_failed)?;

    // Validate
    dag.validate()?;

    println!("✓ Hierarchical workflow created");
    println!("  Total nodes: {}", dag.nodes.len());
    println!("  Total edges: {}", dag.edges.len());

    // Analyze hierarchy
    let parent_nodes: Vec<_> = dag.nodes.values()
        .filter(|n| n.metadata.contains_key("parent"))
        .collect();
    println!("  Sub-states: {}", parent_nodes.len());

    // Statistics
    let stats = dag.statistics()?;
    println!("\nWorkflow statistics:");
    println!("  Max depth: {}", stats.max_depth);
    println!("  Is connected: {}", stats.is_connected);

    // Export
    let config = SwarmExportConfig::default()
        .with_workflow_name("application_deployment")
        .with_description("Hierarchical deployment workflow with sub-processes")
        .with_agent("deployment_orchestrator", "claude-3-opus")
        .with_agent("build_agent", "claude-3-sonnet")
        .with_agent("compiler_agent", "claude-3-sonnet")
        .with_agent("test_runner_agent", "claude-3-sonnet")
        .with_agent("packaging_agent", "claude-3-sonnet")
        .with_agent("deployment_agent", "claude-3-opus")
        .with_agent("environment_deployer", "claude-3-sonnet")
        .with_agent("verification_agent", "claude-3-opus")
        .with_agent("rollback_agent", "claude-3-opus")
        .with_guard("health_check_passed", "context.health_check.status == 'healthy'")
        .with_guard("health_check_failed", "context.health_check.status != 'healthy'")
        .with_author("DevOps Team")
        .with_timeout(10800)
        .with_retries(2, 300);

    let toml = export_dag_to_swarm_toml(&dag, &config)?;

    // Save
    use std::path::Path;
    let output_path = Path::new("examples/dag_workflows/output/hierarchical_workflow.toml");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, toml)?;

    println!("\n✓ Workflow exported to: {:?}", output_path);

    Ok(())
}
