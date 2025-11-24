/// Example 3: Complex Multi-Agent Workflow
///
/// This example demonstrates a realistic multi-agent system with:
/// - Multiple specialized agents
/// - Different edge types
/// - Guards and conditional logic
/// - Resource dependencies
/// - Error handling paths
///
/// Use case: Complex document processing pipeline

use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType};
use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig, ResourceConfig};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = DAG::new("Document Processing Pipeline");
    dag.description = Some("Complex multi-agent document processing with validation and error handling".to_string());

    // Stage 1: Ingestion
    let ingest = DAGNode::new_auto("IngestDocuments")
        .with_description("Receive and validate incoming documents")
        .with_position(200.0, 100.0)
        .with_metadata("agents", serde_json::json!(["ingestion_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["validate_format", "check_virus"]))
        .with_metadata("required_resources", serde_json::json!(["document_storage"]))
        .with_metadata("timeout_seconds", serde_json::json!(300))
        .with_tag("ingestion")
        .with_tag("critical");

    // Stage 2: Classification
    let classify = DAGNode::new_auto("ClassifyDocuments")
        .with_description("Classify documents by type and priority")
        .with_position(450.0, 100.0)
        .with_metadata("agents", serde_json::json!(["classification_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["analyze_content", "determine_type"]))
        .with_metadata("required_resources", serde_json::json!(["ml_model"]))
        .with_tag("classification")
        .with_tag("ml");

    // Stage 3: Parallel Processing Branches
    let extract_text = DAGNode::new_auto("ExtractText")
        .with_description("Extract text content from documents")
        .with_position(200.0, 300.0)
        .with_metadata("agents", serde_json::json!(["text_extraction_agent"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["ocr_engine"]))
        .with_tag("extraction")
        .with_tag("parallel");

    let extract_metadata = DAGNode::new_auto("ExtractMetadata")
        .with_description("Extract metadata and properties")
        .with_position(450.0, 300.0)
        .with_metadata("agents", serde_json::json!(["metadata_agent"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["metadata_parser"]))
        .with_tag("extraction")
        .with_tag("parallel");

    let analyze_sentiment = DAGNode::new_auto("AnalyzeSentiment")
        .with_description("Perform sentiment analysis")
        .with_position(700.0, 300.0)
        .with_metadata("agents", serde_json::json!(["sentiment_agent"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["sentiment_model"]))
        .with_tag("analysis")
        .with_tag("parallel");

    // Stage 4: Validation
    let validate = DAGNode::new_auto("ValidateResults")
        .with_description("Validate extracted data quality")
        .with_position(450.0, 500.0)
        .with_metadata("agents", serde_json::json!(["validation_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["check_completeness", "verify_quality"]))
        .with_metadata("timeout_seconds", serde_json::json!(180))
        .with_tag("validation")
        .with_tag("quality");

    // Stage 5: Error Handling (conditional)
    let handle_errors = DAGNode::new_auto("HandleErrors")
        .with_description("Handle validation errors and retry")
        .with_position(700.0, 500.0)
        .with_metadata("agents", serde_json::json!(["error_handler_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["log_errors", "prepare_retry"]))
        .with_metadata("exit_actions", serde_json::json!(["notify_admin"]))
        .with_tag("error_handling");

    // Stage 6: Storage
    let store = DAGNode::new_auto("StoreResults")
        .with_description("Store processed documents and metadata")
        .with_position(450.0, 700.0)
        .with_metadata("agents", serde_json::json!(["storage_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["prepare_database", "index_documents"]))
        .with_metadata("exit_actions", serde_json::json!(["verify_storage", "update_index"]))
        .with_metadata("required_resources", serde_json::json!(["document_storage", "search_index"]))
        .with_tag("storage")
        .with_tag("critical");

    // Stage 7: Notification
    let notify = DAGNode::new_auto("NotifyCompletion")
        .with_description("Notify stakeholders of completion")
        .with_position(450.0, 900.0)
        .with_metadata("agents", serde_json::json!(["notification_agent"]))
        .with_metadata("entry_actions", serde_json::json!(["generate_summary", "send_notifications"]))
        .with_metadata("required_resources", serde_json::json!(["notification_service"]))
        .with_tag("notification");

    // Store IDs
    let ids = [
        ingest.node_id,
        classify.node_id,
        extract_text.node_id,
        extract_metadata.node_id,
        analyze_sentiment.node_id,
        validate.node_id,
        handle_errors.node_id,
        store.node_id,
        notify.node_id,
    ];

    // Add all nodes
    dag.add_node(ingest)?;
    dag.add_node(classify)?;
    dag.add_node(extract_text)?;
    dag.add_node(extract_metadata)?;
    dag.add_node(analyze_sentiment)?;
    dag.add_node(validate)?;
    dag.add_node(handle_errors)?;
    dag.add_node(store)?;
    dag.add_node(notify)?;

    // Create edges with different types
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])
        .with_label("documents_validated"))?;

    // Branching to parallel tasks
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])
        .with_label("classified_for_text"))?;
    dag.add_edge(DAGEdge::dependency(ids[1], ids[3])
        .with_label("classified_for_metadata"))?;
    dag.add_edge(DAGEdge::dependency(ids[1], ids[4])
        .with_label("classified_for_sentiment"))?;

    // Convergence to validation
    dag.add_edge(DAGEdge::dependency(ids[2], ids[5])
        .with_label("text_extracted"))?;
    dag.add_edge(DAGEdge::dependency(ids[3], ids[5])
        .with_label("metadata_extracted"))?;
    dag.add_edge(DAGEdge::dependency(ids[4], ids[5])
        .with_label("sentiment_analyzed"))?;

    // Conditional paths from validation
    let mut validation_success = DAGEdge::dependency(ids[5], ids[7])
        .with_label("validation_passed");
    validation_success.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["validation_success"])
    );
    dag.add_edge(validation_success)?;

    let mut validation_failed = DAGEdge::dependency(ids[5], ids[6])
        .with_label("validation_failed");
    validation_failed.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["validation_failed"])
    );
    dag.add_edge(validation_failed)?;

    // Error handling back to validation (retry loop)
    dag.add_edge(DAGEdge::soft_dependency(ids[6], ids[5])
        .with_label("retry_validation"))?;

    // Final notification
    dag.add_edge(DAGEdge::dependency(ids[7], ids[8])
        .with_label("storage_complete"))?;

    // Validate
    dag.validate()?;

    println!("✓ Complex multi-agent workflow created");
    println!("  Nodes: {}", dag.nodes.len());
    println!("  Edges: {}", dag.edges.len());

    // Statistics
    let stats = dag.statistics()?;
    println!("\nWorkflow statistics:");
    println!("  Average in-degree: {:.2}", stats.average_in_degree);
    println!("  Average out-degree: {:.2}", stats.average_out_degree);
    println!("  Max depth: {}", stats.max_depth);

    // Export with full configuration
    let mut resources = HashMap::new();
    resources.insert("document_storage".to_string(), ResourceConfig {
        resource_type: "S3Bucket".to_string(),
        connection_string: Some("s3://documents-bucket".to_string()),
        config: HashMap::new(),
    });
    resources.insert("ml_model".to_string(), ResourceConfig {
        resource_type: "MLModel".to_string(),
        connection_string: Some("models/classifier".to_string()),
        config: HashMap::new(),
    });
    resources.insert("ocr_engine".to_string(), ResourceConfig {
        resource_type: "OCREngine".to_string(),
        connection_string: None,
        config: HashMap::new(),
    });

    let mut config = SwarmExportConfig::default()
        .with_workflow_name("document_processing_pipeline")
        .with_description("Complex multi-agent document processing")
        .with_agent("ingestion_agent", "claude-3-opus")
        .with_agent("classification_agent", "claude-3-opus")
        .with_agent("text_extraction_agent", "claude-3-sonnet")
        .with_agent("metadata_agent", "claude-3-sonnet")
        .with_agent("sentiment_agent", "claude-3-sonnet")
        .with_agent("validation_agent", "claude-3-opus")
        .with_agent("error_handler_agent", "claude-3-haiku")
        .with_agent("storage_agent", "claude-3-sonnet")
        .with_agent("notification_agent", "claude-3-haiku")
        .with_guard("validation_success", "context.validation.passed == true")
        .with_guard("validation_failed", "context.validation.passed == false")
        .with_author("Document Processing Team")
        .with_timeout(7200)
        .with_retries(3, 60);

    for (name, resource) in resources {
        config = config.with_resource(name, resource);
    }

    let toml = export_dag_to_swarm_toml(&dag, &config)?;

    // Save
    use std::path::Path;
    let output_path = Path::new("examples/dag_workflows/output/complex_multiagent.toml");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, toml)?;

    println!("\n✓ Workflow exported to: {:?}", output_path);

    Ok(())
}
