use descartes_core::{IterativeLoop, IterativeLoopConfig, IterativeExitReason};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let test_dir = PathBuf::from("/tmp/loop-test");
    std::fs::create_dir_all(&test_dir)?;
    let state_file = test_dir.join(".descartes/loop-state.json");
    
    // Clean up any previous state
    let _ = std::fs::remove_file(&state_file);
    
    println!("=== Manual Verification Test ===\n");
    
    // Test 1: State file creation
    println!("1. Testing state file creation...");
    let config = IterativeLoopConfig {
        command: "echo".to_string(),
        args: vec![],
        prompt: "<promise>DONE</promise>".to_string(),
        completion_promise: Some("DONE".to_string()),
        max_iterations: Some(3),
        working_directory: Some(test_dir.clone()),
        state_file: Some(state_file.clone()),
        include_iteration_context: false,
        ..Default::default()
    };
    
    let mut loop_exec = IterativeLoop::new(config).await?;
    let result = loop_exec.execute().await?;
    
    println!("   Exit reason: {:?}", result.exit_reason);
    println!("   Iterations: {}", result.iterations_completed);
    
    // Check state file exists
    if state_file.exists() {
        println!("   ✓ State file created at: {}", state_file.display());
        let content = std::fs::read_to_string(&state_file)?;
        let state: serde_json::Value = serde_json::from_str(&content)?;
        println!("   ✓ State file contains valid JSON");
        println!("   ✓ Completed: {}", state["completed"]);
        println!("   ✓ Iteration: {}", state["iteration"]);
    } else {
        println!("   ✗ State file NOT created!");
        return Err(anyhow::anyhow!("State file not created"));
    }
    
    // Test 2: Resume functionality (create a state that's not complete)
    println!("\n2. Testing resume functionality...");
    
    // Write a partial state
    let partial_state = r#"{
        "version": "1.0.0",
        "config": {
            "command": "echo",
            "args": [],
            "prompt": "<promise>RESUMED</promise>",
            "completion_promise": "RESUMED",
            "max_iterations": 5,
            "include_iteration_context": false,
            "backend": {"backend_type": "generic", "prompt_mode": "arg", "environment": {}, "output_format": "text"},
            "git": {"auto_commit": false, "commit_template": "iter({iteration}): {summary}", "create_branch": false, "branch_template": "loop/{timestamp}"}
        },
        "iteration": 2,
        "completed": false,
        "started_at": "2025-12-28T00:00:00Z",
        "iterations": []
    }"#;
    
    std::fs::create_dir_all(state_file.parent().unwrap())?;
    std::fs::write(&state_file, partial_state)?;
    println!("   Created partial state at iteration 2");
    
    let mut resumed = IterativeLoop::resume(state_file.clone()).await?;
    println!("   ✓ Resumed from state file");
    println!("   Current iteration: {}", resumed.current_iteration());
    
    let result = resumed.execute().await?;
    println!("   ✓ Execution completed");
    println!("   Exit reason: {:?}", result.exit_reason);
    println!("   Total iterations: {}", result.iterations_completed);
    
    // Test 3: Verify completion promise detection works
    println!("\n3. Testing completion promise detection...");
    let _ = std::fs::remove_file(&state_file);
    
    let config = IterativeLoopConfig {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo 'Working...'; echo '<promise>TASK_COMPLETE</promise>'".to_string()],
        prompt: "".to_string(),
        completion_promise: Some("TASK_COMPLETE".to_string()),
        max_iterations: Some(10),
        working_directory: Some(test_dir.clone()),
        state_file: Some(state_file.clone()),
        include_iteration_context: false,
        ..Default::default()
    };
    
    let mut loop_exec = IterativeLoop::new(config).await?;
    let result = loop_exec.execute().await?;
    
    println!("   Exit reason: {:?}", result.exit_reason);
    assert_eq!(result.exit_reason, IterativeExitReason::CompletionPromiseDetected);
    println!("   ✓ Completion promise correctly detected");
    
    println!("\n=== All Manual Verification Tests Passed! ===");
    Ok(())
}
