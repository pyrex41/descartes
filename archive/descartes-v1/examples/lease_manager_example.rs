/// Example demonstrating the TTL-Based File Leasing System
///
/// This example shows how to:
/// 1. Initialize the lease manager
/// 2. Acquire a lease on a file
/// 3. Renew a lease
/// 4. Release a lease
/// 5. Check lock status
/// 6. Clean up expired leases

use descartes_core::{
    LeaseAcquisitionRequest, LeaseManager, LeaseReleaseRequest, LeaseRenewalRequest,
    SqliteLeaseManager,
};
use std::path::PathBuf;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("TTL-Based File Leasing System Example");
    println!("======================================\n");

    // Step 1: Initialize the lease manager
    println!("Step 1: Initializing lease manager...");
    let db_path = PathBuf::from("/tmp/leases.db");
    let manager = SqliteLeaseManager::new(db_path).await?;
    manager.initialize().await?;
    println!("Lease manager initialized successfully\n");

    // Step 2: Create some test agents
    let agent_1 = Uuid::new_v4();
    let agent_2 = Uuid::new_v4();
    let file_path = PathBuf::from("/tmp/important_file.txt");

    println!("Agent 1 ID: {}", agent_1);
    println!("Agent 2 ID: {}\n", agent_2);

    // Step 3: Acquire a lease for agent 1
    println!("Step 2: Agent 1 acquiring lease...");
    let acquisition_request = LeaseAcquisitionRequest {
        file_path: file_path.clone(),
        agent_id: agent_1,
        ttl_seconds: 60,
        max_renewals: 3,
        timeout_ms: Some(5000),
        blocking: false,
    };

    let response = manager.acquire_lease(acquisition_request).await?;
    if response.success {
        let lease = response.lease.unwrap();
        println!("✓ Lease acquired successfully");
        println!("  Lease ID: {}", lease.id);
        println!("  Status: {}", lease.status);
        println!("  TTL: {} seconds", lease.ttl.num_seconds());
        println!("  Renewals: {}/{}\n", lease.renewal_count, lease.max_renewals);

        // Step 4: Check if file is locked
        println!("Step 3: Checking if file is locked...");
        let is_locked = manager.is_file_locked(&file_path).await?;
        println!("File '{}' is locked: {}\n", file_path.display(), is_locked);

        // Step 5: Try to acquire the same file as agent 2 (should fail)
        println!("Step 4: Agent 2 attempting to acquire same file...");
        let conflicting_request = LeaseAcquisitionRequest {
            file_path: file_path.clone(),
            agent_id: agent_2,
            ttl_seconds: 60,
            max_renewals: 3,
            timeout_ms: Some(2000),
            blocking: false,
        };

        let conflict_response = manager.acquire_lease(conflicting_request).await?;
        if !conflict_response.success {
            println!("✓ Expected: Agent 2 could not acquire lock");
            println!("  Error: {}\n", conflict_response.error.unwrap());
        }

        // Step 6: Renew the lease for agent 1
        println!("Step 5: Agent 1 renewing lease...");
        let renewal_request = LeaseRenewalRequest {
            lease_id: lease.id,
            agent_id: agent_1,
            new_ttl_seconds: Some(120),
        };

        let renewal_response = manager.renew_lease(renewal_request).await?;
        if renewal_response.success {
            let renewed_lease = renewal_response.lease.unwrap();
            println!("✓ Lease renewed successfully");
            println!("  New TTL: {} seconds", renewed_lease.ttl.num_seconds());
            println!("  Renewals: {}/{}\n", renewed_lease.renewal_count, renewed_lease.max_renewals);

            // Step 7: Get all leases held by agent 1
            println!("Step 6: Getting all leases for Agent 1...");
            let agent_leases = manager.get_agent_leases(&agent_1).await?;
            println!("Agent 1 holds {} lease(s)\n", agent_leases.len());
            for lease in agent_leases {
                println!("  - File: {}", lease.file_path.display());
                println!("    Status: {}", lease.status);
                println!("    Remaining TTL: {:?}", lease.time_remaining());
            }
            println!();

            // Step 8: Get all leases for the file
            println!("Step 7: Getting all leases for file...");
            let file_leases = manager.get_file_leases(&file_path).await?;
            println!("File '{}' has {} lease(s)\n", file_path.display(), file_leases.len());

            // Step 9: Release the lease
            println!("Step 8: Agent 1 releasing lease...");
            let release_request = LeaseReleaseRequest {
                lease_id: lease.id,
                agent_id: agent_1,
            };

            let release_response = manager.release_lease(release_request).await?;
            if release_response.success {
                println!("✓ Lease released successfully\n");
            }

            // Step 10: Check if file is still locked
            println!("Step 9: Checking if file is still locked...");
            let is_still_locked = manager.is_file_locked(&file_path).await?;
            println!("File '{}' is locked: {}\n", file_path.display(), is_still_locked);

            // Step 11: Now agent 2 should be able to acquire the lease
            println!("Step 10: Agent 2 attempting to acquire file again...");
            let agent2_request = LeaseAcquisitionRequest {
                file_path: file_path.clone(),
                agent_id: agent_2,
                ttl_seconds: 60,
                max_renewals: 2,
                timeout_ms: Some(5000),
                blocking: false,
            };

            let agent2_response = manager.acquire_lease(agent2_request).await?;
            if agent2_response.success {
                let agent2_lease = agent2_response.lease.unwrap();
                println!("✓ Agent 2 successfully acquired lease");
                println!("  Lease ID: {}\n", agent2_lease.id);

                // Clean up agent 2's lease
                let cleanup_request = LeaseReleaseRequest {
                    lease_id: agent2_lease.id,
                    agent_id: agent_2,
                };
                let _ = manager.release_lease(cleanup_request).await;
            }
        }
    }

    // Step 12: Clean up expired leases
    println!("Step 11: Running cleanup for expired leases...");
    let cleaned_up = manager.cleanup_expired_leases().await?;
    println!("Cleaned up {} expired lease(s)\n", cleaned_up);

    // Step 13: Get all remaining leases
    println!("Step 12: Listing all remaining leases...");
    let all_leases = manager.get_all_leases().await?;
    println!("Total leases in system: {}\n", all_leases.len());
    for lease in all_leases {
        println!(
            "  - ID: {} | File: {} | Agent: {} | Status: {}",
            lease.id,
            lease.file_path.display(),
            lease.agent_id,
            lease.status
        );
    }

    println!("\nExample completed successfully!");
    Ok(())
}
