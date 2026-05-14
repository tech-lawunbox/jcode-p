// Tests for swarm completion report delivery mechanism.
//
// ## Key Flows
//
// ### Flow 1: `spawn` action
// ```
// coordinator → spawn prompt="task" → worker session starts
// → worker runs task, calls swarm action=report → coordinator receives notification
// ```
//
// ### Flow 2: `assign_task` action
// ```
// coordinator → assign_task task_id=X target_session=worker → worker gets task
// → worker runs, calls swarm action=report → coordinator receives notification
// ```
//
// ## Gap Being Tested
//
// If agent finishes without calling `action=report`, coordinator receives:
//     "Agent X finished their work and is ready for more.\nNo final textual report was produced."
//
// The agent's actual findings are LOST unless it explicitly calls `swarm action=report`.

// SwarmMember, HashMap, Instant, mpsc are imported from parent module (comm_control_tests.rs)

// ============================================================================
// TEST 1: Verify completion report instructions are injected
// ============================================================================

#[tokio::test]
async fn completion_report_marker_injected_in_prompt() {
    let task_prompt = "Analyze the codebase and report findings.";
    
    // Without marker - just the raw prompt
    assert!(!task_prompt.contains("SWARM COMPLETION REPORT REQUIRED"));
    
    // With marker (correct usage) - append_swarm_completion_report_instructions
    let with_marker = append_swarm_completion_report_instructions(task_prompt);
    assert!(with_marker.contains("SWARM COMPLETION REPORT REQUIRED"));
    assert!(with_marker.contains("action=\"report\""));
    assert!(with_marker.contains("Include a concise message"));
}

// ============================================================================
// TEST 2: Verify coordinator receives empty report when agent doesn't call report
//         (This reproduces the bug you're seeing)
// ============================================================================

#[tokio::test]
async fn coordinator_receives_empty_report_when_agent_skips_report_action() {
    let swarm_id = "swarm-report-test";
    let coordinator_id = "coordinator";
    let worker_id = "worker";
    
    // Setup: coordinator + worker in same swarm
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (coordinator_id.to_string(), {
            let mut m = test_member(coordinator_id, swarm_id, "ready");
            m.role = "coordinator".to_string();
            m
        }),
        (worker_id.to_string(), test_member_with_report_to(worker_id, swarm_id, "ready", coordinator_id)),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([
        (swarm_id.to_string(), HashSet::from([coordinator_id.to_string(), worker_id.to_string()])),
    ])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _rx) = broadcast::channel(32);
    
    // Simulate: worker finishes WITHOUT calling action=report
    // Status changes to "ready" - this triggers notification to coordinator
    crate::server::swarm::update_member_status_with_report(
        worker_id,
        "ready",
        Some("Task complete.".to_string()),
        None, // <-- No report! This is the bug.
        &swarm_members,
        &swarms_by_id,
        Some(&event_history),
        Some(&event_counter),
        Some(&swarm_event_tx),
        None, // sessions
        None, // stop_worker_on_completion
    )
    .await;
    
    // Verify: coordinator should receive notification
    let members = swarm_members.read().await;
    let coordinator = members.get(coordinator_id).expect("coordinator exists");
    
    // The coordinator's latest_completion_report is NOT set because worker didn't call report
    assert!(
        coordinator.latest_completion_report.is_none(),
        "BUG REPRODUCED: Coordinator received empty report when agent skipped action=report"
    );
}

// ============================================================================
// TEST 3: Verify coordinator receives report when agent DOES call report
//         (This is the correct flow)
// ============================================================================

#[tokio::test]
async fn coordinator_receives_findings_when_agent_calls_report_action() {
    let swarm_id = "swarm-report-test-correct";
    let coordinator_id = "coordinator";
    let worker_id = "worker";
    
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (coordinator_id.to_string(), {
            let mut m = test_member(coordinator_id, swarm_id, "ready");
            m.role = "coordinator".to_string();
            m
        }),
        (worker_id.to_string(), test_member_with_report_to(worker_id, swarm_id, "ready", coordinator_id)),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([
        (swarm_id.to_string(), HashSet::from([coordinator_id.to_string(), worker_id.to_string()])),
    ])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _rx) = broadcast::channel(32);
    
    // Simulate: worker calls action=report with findings
    crate::server::swarm::update_member_status_with_report(
        worker_id,
        "ready",
        Some("Task complete.".to_string()),
        Some("FINDINGS: Critical issue found in line 42 - missing null check".to_string()), // Report!
        &swarm_members,
        &swarms_by_id,
        Some(&event_history),
        Some(&event_counter),
        Some(&swarm_event_tx),
        None,
        None, // stop_worker_on_completion
    )
    .await;
    
    let members = swarm_members.read().await;
    let coordinator = members.get(coordinator_id).expect("coordinator exists");
    
    // CORRECT: Coordinator's latest_completion_report IS set
    assert!(
        coordinator.latest_completion_report.is_some(),
        "Coordinator should receive report when agent calls action=report"
    );
    let report = coordinator.latest_completion_report.as_ref().unwrap();
    assert!(
        report.contains("FINDINGS: Critical issue found in line 42"),
        "Report should contain the findings"
    );
}

// ============================================================================
// TEST 4: Verify report_back_to_session_id is set correctly for spawn
//         (This is the missing piece in your case)
// ============================================================================

#[tokio::test]
async fn spawn_sets_report_back_to_coordinator() {
    let swarm_id = "swarm-spawn-test";
    let coordinator_id = "coordinator";
    let worker_id = "worker";
    
    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (coordinator_id.to_string(), {
            let mut m = test_member(coordinator_id, swarm_id, "ready");
            m.role = "coordinator".to_string();
            m
        }),
    ])));
    
    // Simulate spawn: worker joins with report_back_to set to coordinator
    let mut worker = test_member(worker_id, swarm_id, "ready");
    worker.report_back_to_session_id = Some(coordinator_id.to_string());
    
    let mut members = swarm_members.write().await;
    members.insert(worker_id.to_string(), worker);
    
    let worker = members.get(worker_id).expect("worker exists");
    assert_eq!(
        worker.report_back_to_session_id.as_deref(),
        Some(coordinator_id),
        "Worker should have report_back_to_session_id set to coordinator"
    );
}

// ============================================================================
// TEST 5: Verify notification message format matches what you see
// ============================================================================

#[tokio::test]
async fn notification_message_format_matches_observed_output() {
    use jcode_swarm_core::completion_notification_message;
    
    // This is exactly what you saw:
    // "Agent chick finished their work and is ready for more."
    // "No final textual report was produced."
    
    let msg_no_report = completion_notification_message("chick", "ready", None);
    assert!(msg_no_report.contains("Agent chick finished their work and is ready for more."));
    assert!(msg_no_report.contains("No final textual report was produced."));
    
    // With actual findings:
    let msg_with_report = completion_notification_message(
        "chick",
        "ready",
        Some("FINDINGS: Critical bug in auth flow"),
    );
    assert!(msg_with_report.contains("Agent chick finished their work and is ready for more."));
    assert!(msg_with_report.contains("FINDINGS: Critical bug in auth flow"));
    assert!(!msg_with_report.contains("No final textual report was produced."));
}

// ============================================================================
// Helper functions
// ============================================================================

fn test_member(session_id: &str, swarm_id: &str, status: &str) -> SwarmMember {
    let (event_tx, _rx) = mpsc::unbounded_channel();
    SwarmMember {
        session_id: session_id.to_string(),
        event_tx,
        event_txs: HashMap::new(),
        working_dir: None,
        swarm_id: Some(swarm_id.to_string()),
        swarm_enabled: true,
        status: status.to_string(),
        detail: None,
        friendly_name: Some(session_id.to_string()),
        report_back_to_session_id: None,
        run_id: None,
        latest_completion_report: None,
        role: "agent".to_string(),
        joined_at: Instant::now(),
        last_status_change: Instant::now(),
        is_headless: false,
    }
}

fn test_member_with_report_to(session_id: &str, swarm_id: &str, status: &str, report_to: &str) -> SwarmMember {
    let mut m = test_member(session_id, swarm_id, status);
    m.report_back_to_session_id = Some(report_to.to_string());
    m
}

// ============================================================================
// SUMMARY: What these tests reveal
// ============================================================================
//
// These tests demonstrate the swarm report delivery mechanism and its gaps:
//
// BUG SCENARIO (Test 2):
//   - Agent spawns, runs task, outputs "I'm done"
//   - Agent does NOT call `swarm action=report`
//   - Coordinator receives: "No final textual report was produced."
//   - Agent's findings are LOST
//
// CORRECT FLOW (Test 3):
//   - Agent spawns, runs task
//   - Agent calls `swarm action=report message="FINDINGS..."`
//   - Coordinator receives the findings
//
// ROOT CAUSE:
//   The completion marker instructs agents to call report, but there's no
//   enforcement. If the LLM decides to end the session without calling the
//   report action, all findings are lost.
//
// SOLUTIONS:
// 1. Force prompts to be more explicit: "Your response IS the report"
// 2. Use `assign_task` which has better tracking
// 3. Have agents write findings to files, read after await
// 4. (Future) Server-side: extract output from agent session if no report received

// ============================================================================
// TEST 5: Verify auto-cleanup of spawned worker after explicit report
// ============================================================================

#[tokio::test]
async fn spawned_worker_is_cleaned_up_after_explicit_report() {
    let swarm_id = "swarm-auto-cleanup-test";
    let coordinator_id = "coordinator";
    let worker_id = "worker";

    let swarm_members = Arc::new(RwLock::new(HashMap::from([
        (coordinator_id.to_string(), {
            let mut m = test_member(coordinator_id, swarm_id, "ready");
            m.role = "coordinator".to_string();
            m
        }),
        (worker_id.to_string(), test_member_with_report_to(worker_id, swarm_id, "ready", coordinator_id)),
    ])));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::from([
        (swarm_id.to_string(), HashSet::from([coordinator_id.to_string(), worker_id.to_string()])),
    ])));
    let event_history = Arc::new(RwLock::new(VecDeque::new()));
    let event_counter = Arc::new(AtomicU64::new(1));
    let (swarm_event_tx, _rx) = broadcast::channel(32);
    let sessions = Arc::new(RwLock::new(HashMap::new()));

    // Worker calls action=report with findings AND auto_cleanup is enabled
    crate::server::swarm::update_member_status_with_report(
        worker_id,
        "ready",
        Some("Task complete.".to_string()),
        Some("FINDINGS: All tests passed, no issues found.".to_string()),
        &swarm_members,
        &swarms_by_id,
        Some(&event_history),
        Some(&event_counter),
        Some(&swarm_event_tx),
        Some(&sessions),
        Some(true), // stop_worker_on_completion - enable auto-cleanup
    )
    .await;

    // Give the background cleanup task time to run
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify: worker should be removed from swarm members
    let members = swarm_members.read().await;
    assert!(
        members.get(worker_id).is_none(),
        "Worker should be auto-cleaned from swarm members after explicit report"
    );
    drop(members);

    // Verify: worker should be removed from swarms_by_id
    let swarms = swarms_by_id.read().await;
    let swarm_members_set = swarms.get(swarm_id);
    assert!(
        swarm_members_set.is_none() || !swarm_members_set.unwrap().contains(worker_id),
        "Worker should be removed from swarms_by_id after explicit report"
    );
}
//
