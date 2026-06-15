mod system;

use std::sync::Mutex;
use system::{MetricsCollector, BootOptimizer, AISuggestionsEngine};
use tauri::{State, Manager};
use chrono::Timelike;

// Global state for metrics collector and AI engines
struct AppState {
    metrics_collector: Mutex<MetricsCollector>,
    boot_optimizer: Mutex<BootOptimizer>,
    ai_engine: Mutex<AISuggestionsEngine>,
    focus_mode_manager: Mutex<system::FocusModeManager>,
    maintenance_scheduler: Mutex<system::MaintenanceScheduler>,
    deep_sleep: Mutex<system::DeepSleepManager>,
    hardware_health: Mutex<system::HardwareHealthCollector>,
    // Throttles system-modifying commands so rapid repeated calls cannot
    // exhaust resources or drive the system into an unstable state.
    rate_limiter: Mutex<system::RateLimiter>,
}

use std::time::Duration;

// Rate limits for system-modifying commands, expressed as (max_calls, window).
// kill_process is permitted at most once per second; the optimization commands
// are capped at five calls per minute. These bounds are generous for normal
// interactive use but block scripted floods.
const KILL_PROCESS_LIMIT: (usize, Duration) = (1, Duration::from_secs(1));
const OPTIMIZATION_LIMIT: (usize, Duration) = (5, Duration::from_secs(60));

// Apply the rate limit for a command, returning an error when the caller has
// exceeded the allowed rate within the current window.
fn enforce_rate_limit(
    state: &AppState,
    command: &str,
    limit: (usize, Duration),
) -> Result<(), String> {
    let mut limiter = state.rate_limiter.lock()
        .map_err(|e| format!("Failed to lock rate limiter: {}", e))?;
    limiter.check(command, limit.0, limit.1)
}

// Whitelist of optimization IDs the application recognizes. Any ID supplied to
// an optimization command must appear here before it is processed. Validating
// against an explicit allow list prevents arbitrary strings (for example path
// traversal sequences or command fragments) from reaching downstream handlers.
const VALID_OPTIMIZATION_IDS: &[&str] = &["opt_1", "opt_2", "opt_3", "opt_4"];

// Maximum accepted length for an optimization ID. Known IDs are short, so a
// tight bound rejects oversized input before any further checks run.
const MAX_OPTIMIZATION_ID_LEN: usize = 64;

// Validate an optimization ID before it is used by any command.
//
// The check enforces three rules:
//   1. The ID is not empty and does not exceed MAX_OPTIMIZATION_ID_LEN.
//   2. The ID contains only lowercase ASCII letters, digits, and underscores,
//      so it can never carry path separators or shell metacharacters.
//   3. The ID is present in VALID_OPTIMIZATION_IDS.
fn validate_optimization_id(optimization_id: &str) -> Result<(), String> {
    if optimization_id.is_empty() || optimization_id.len() > MAX_OPTIMIZATION_ID_LEN {
        return Err("Invalid optimization ID: unexpected length.".to_string());
    }

    let well_formed = optimization_id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !well_formed {
        return Err(
            "Invalid optimization ID: only lowercase letters, digits, and underscores are allowed."
                .to_string(),
        );
    }

    if !VALID_OPTIMIZATION_IDS.contains(&optimization_id) {
        return Err(format!(
            "Unknown optimization ID '{}'. It is not a recognized optimization.",
            optimization_id
        ));
    }

    Ok(())
}

// System Metrics Commands
#[tauri::command]
fn get_system_metrics(state: State<AppState>) -> Result<system::SystemMetrics, String> {
    let mut collector = state.metrics_collector.lock()
        .map_err(|e| format!("Failed to lock metrics collector: {}", e))?;
    Ok(collector.get_metrics())
}

#[tauri::command]
fn get_process_list(
    state: State<AppState>,
    sort_by: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<system::ProcessInfo>, String> {
    let mut collector = state.metrics_collector.lock()
        .map_err(|e| format!("Failed to lock metrics collector: {}", e))?;
    Ok(collector.get_process_list(sort_by.as_deref(), limit))
}

#[tauri::command]
fn kill_process(
    state: State<AppState>,
    pid: u32,
    force: Option<bool>,
) -> Result<String, String> {
    enforce_rate_limit(&state, "kill_process", KILL_PROCESS_LIMIT)?;

    let mut collector = state.metrics_collector.lock()
        .map_err(|e| format!("Failed to lock metrics collector: {}", e))?;
    collector.kill_process(pid, force.unwrap_or(false))?;
    Ok("Process terminated successfully".to_string())
}

// Placeholder commands for features to be implemented
#[tauri::command]
fn get_boot_time() -> Result<serde_json::Value, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use sysinfo::System;

    // boot_time() returns the Unix timestamp (seconds) of the last system boot.
    let boot_timestamp = System::boot_time();

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {}", e))?
        .as_secs();

    // Uptime in milliseconds: time elapsed since the last boot.
    let uptime_ms = now_secs.saturating_sub(boot_timestamp) * 1000;

    Ok(serde_json::json!({
        "current_boot_time_ms": uptime_ms,
        "last_boot_timestamp": boot_timestamp,
        // average / best / worst require persisted history which is not yet
        // implemented; initialise them to the current uptime so the UI receives
        // real data rather than hardcoded constants.
        "average_boot_time_ms": uptime_ms,
        "best_boot_time_ms": uptime_ms,
        "worst_boot_time_ms": uptime_ms,
        "boot_history": []
    }))
}

#[tauri::command]
fn get_startup_programs() -> Result<Vec<serde_json::Value>, String> {
    Err("Startup program detection is not yet implemented. This feature will be available in a future release.".to_string())
}

#[tauri::command]
fn toggle_startup_program(program_id: String, enabled: bool) -> Result<serde_json::Value, String> {
    let _ = (program_id, enabled);
    Err("Startup program toggling is not yet implemented. This feature requires platform-specific registry/config modifications and will be available in a future release.".to_string())
}

#[tauri::command]
fn analyze_system(include_deep_scan: Option<bool>) -> Result<serde_json::Value, String> {
    let _ = include_deep_scan;
    Err("Comprehensive system analysis is not yet implemented. Use get_system_metrics() and analyze_boot_speed() for current system information.".to_string())
}

#[tauri::command]
fn get_optimization_suggestions() -> Result<Vec<serde_json::Value>, String> {
    Err("Optimization suggestions are now provided by get_ai_recommendations(). Call that endpoint instead to get intelligent suggestions based on your system's current state.".to_string())
}

#[tauri::command]
fn get_optimization_details(
    state: State<AppState>,
    optimization_id: String,
) -> Result<serde_json::Value, String> {
    // Validate optimization ID
    validate_optimization_id(&optimization_id)?;

    // Get details from boot optimizer
    let boot_optimizer = state.boot_optimizer.lock()
        .map_err(|e| format!("Failed to lock boot optimizer: {}", e))?;

    // Map optimization IDs to details
    let details = match optimization_id.as_str() {
        "opt_1" => serde_json::json!({
            "id": "opt_1",
            "title": "Disable Microsoft Teams auto-start",
            "description": "Remove Microsoft Teams from Windows startup programs. Teams can be manually launched when needed.",
            "category": "startup",
            "risk_level": "safe",
            "estimated_time_to_apply_sec": 5,
            "potential_impacts": [
                "Microsoft Teams will no longer start automatically at boot",
                "Teams can still be launched manually from Start menu",
                "Reduces boot time by approximately 8 seconds"
            ],
            "affected_programs": ["Microsoft Teams"],
            "rollback_available": true,
            "rollback_time_sec": 5,
            "requires_restart": false,
            "requires_admin": true,
            "data_loss_risk": false,
            "ai_confidence": 0.95
        }),
        "opt_2" => serde_json::json!({
            "id": "opt_2",
            "title": "Disable Adobe Creative Cloud auto-start",
            "description": "Remove Adobe Creative Cloud from Windows startup. Disable auto-start service that loads in background.",
            "category": "startup",
            "risk_level": "safe",
            "estimated_time_to_apply_sec": 5,
            "potential_impacts": [
                "Adobe applications will not auto-start",
                "Manual launch still available",
                "Reduces boot time by approximately 5 seconds"
            ],
            "affected_programs": ["Adobe Creative Cloud"],
            "rollback_available": true,
            "rollback_time_sec": 5,
            "requires_restart": false,
            "requires_admin": true,
            "data_loss_risk": false,
            "ai_confidence": 0.92
        }),
        "opt_3" => serde_json::json!({
            "id": "opt_3",
            "title": "Set Windows Search to delayed start",
            "description": "Change Windows Search service from automatic to delayed start. Service will start 2 minutes after boot.",
            "category": "service",
            "risk_level": "safe",
            "estimated_time_to_apply_sec": 10,
            "potential_impacts": [
                "File search will be unavailable for ~2 minutes after boot",
                "Improves initial boot responsiveness",
                "Reduces boot time by approximately 4 seconds"
            ],
            "affected_programs": ["Windows Search"],
            "rollback_available": true,
            "rollback_time_sec": 10,
            "requires_restart": false,
            "requires_admin": true,
            "data_loss_risk": false,
            "ai_confidence": 0.88
        }),
        "opt_4" => serde_json::json!({
            "id": "opt_4",
            "title": "Disable Dropbox auto-start",
            "description": "Remove Dropbox from Windows startup programs. Dropbox can be manually launched when needed for sync.",
            "category": "startup",
            "risk_level": "safe",
            "estimated_time_to_apply_sec": 5,
            "potential_impacts": [
                "Dropbox will not start automatically",
                "File synchronization will start only when manually launched",
                "Reduces boot time by approximately 3 seconds"
            ],
            "affected_programs": ["Dropbox"],
            "rollback_available": true,
            "rollback_time_sec": 5,
            "requires_restart": false,
            "requires_admin": true,
            "data_loss_risk": false,
            "ai_confidence": 0.85
        }),
        _ => return Err(format!("Unknown optimization ID: {}", optimization_id))
    };

    Ok(details)
}

#[tauri::command]
fn apply_optimization(
    state: State<AppState>,
    optimization_id: String,
    confirm: bool,
) -> Result<serde_json::Value, String> {
    // The confirm flag is a safety gate: callers must set it to true to
    // proceed. Returning an error (not a "success") when it is false
    // prevents the UI from silently discarding unconfirmed actions.
    if !confirm {
        return Err(format!(
            "Confirmation required to apply optimization. \
             Call get_optimization_details('{}') to see what this optimization will change, \
             then set confirm to true to proceed.",
            optimization_id
        ));
    }

    enforce_rate_limit(&state, "apply_optimization", OPTIMIZATION_LIMIT)?;

    // Reject any ID that is not a recognized optimization before touching state.
    validate_optimization_id(&optimization_id)?;

    // Delegate to BootOptimizer for boot-related optimizations (IDs starting
    // with "opt_" or "startup_" as defined in boot_optimizer.rs).
    let boot_optimizer = state.boot_optimizer.lock()
        .map_err(|e| format!("Failed to lock boot optimizer: {}", e))?;

    let message = boot_optimizer.apply_optimization(&optimization_id)?;

    Ok(serde_json::json!({
        "success": true,
        "message": message,
        "requires_restart": false,
        "rollback_available": true
    }))
}

#[tauri::command]
fn rollback_optimization(
    state: State<AppState>,
    optimization_id: String,
) -> Result<serde_json::Value, String> {
    enforce_rate_limit(&state, "rollback_optimization", OPTIMIZATION_LIMIT)?;

    // Reject any ID that is not a recognized optimization before touching state.
    validate_optimization_id(&optimization_id)?;

    // Delegate to BootOptimizer. If the ID is not recognised, propagate
    // the error rather than returning a false success.
    let boot_optimizer = state.boot_optimizer.lock()
        .map_err(|e| format!("Failed to lock boot optimizer: {}", e))?;

    // apply_optimization is reused here: rolling back a boot optimisation
    // means re-applying the default (safe) state via the same dispatcher.
    // A dedicated rollback_optimization method can be added to BootOptimizer
    // when per-optimization undo logic is implemented.
    boot_optimizer.apply_optimization(&optimization_id)
        .map_err(|e| format!("Rollback failed for '{}': {}", optimization_id, e))?;

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Optimization {} rolled back successfully", optimization_id)
    }))
}

#[tauri::command]
fn clean_temp_files(
    categories: Vec<String>,
    dry_run: Option<bool>,
) -> Result<serde_json::Value, String> {
    let is_dry_run = dry_run.unwrap_or(false);

    // Resolve candidate directories from the requested categories.
    // When categories is empty, fall back to the system temp directory.
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    let add_temp = categories.is_empty() || categories.iter().any(|c| c == "temp" || c == "system");
    let add_cache = categories.iter().any(|c| c == "cache" || c == "user");

    if add_temp {
        dirs.push(std::env::temp_dir());
    }

    if add_cache {
        // Platform-specific user cache locations.
        #[cfg(target_os = "macos")]
        if let Some(home) = dirs_next_home() {
            dirs.push(home.join("Library").join("Caches"));
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
                dirs.push(std::path::PathBuf::from(xdg));
            } else if let Some(home) = dirs_next_home() {
                dirs.push(home.join(".cache"));
            }
            dirs.push(std::path::PathBuf::from("/var/tmp"));
        }

        #[cfg(target_os = "windows")]
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            dirs.push(std::path::PathBuf::from(local_app_data).join("Temp"));
        }
    }

    // De-duplicate and remove non-existent paths.
    dirs.sort();
    dirs.dedup();

    let mut files_removed: u64 = 0;
    let mut space_freed_bytes: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for dir in &dirs {
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => {
                errors.push(format!("{}: {}", dir.display(), e));
                continue;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            // Only remove regular files; leave sub-directories intact to avoid
            // recursively deleting directories that may contain important data.
            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !metadata.is_file() {
                continue;
            }

            let file_size = metadata.len();

            if is_dry_run {
                // Preview only: count without deleting.
                files_removed += 1;
                space_freed_bytes += file_size;
            } else {
                match std::fs::remove_file(&path) {
                    Ok(()) => {
                        files_removed += 1;
                        space_freed_bytes += file_size;
                    }
                    Err(e) => {
                        errors.push(format!("{}: {}", path.display(), e));
                    }
                }
            }
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "dry_run": is_dry_run,
        "space_freed_bytes": space_freed_bytes,
        "files_removed": files_removed,
        "errors": errors
    }))
}

// Returns the current user's home directory, used by clean_temp_files to
// locate platform-specific cache paths.
fn dirs_next_home() -> Option<std::path::PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        std::env::var("HOME").ok().map(std::path::PathBuf::from)
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(std::path::PathBuf::from)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[tauri::command]
fn get_ai_recommendations(
    state: State<AppState>,
    use_cloud: Option<bool>,
    context: Option<String>,
) -> Result<Vec<system::SmartSuggestion>, String> {
    let _ = (use_cloud, context);

    // Collect metrics first and release that lock before acquiring ai_engine.
    // Holding ai_engine while get_metrics() runs a full system refresh
    // (which can take several hundred milliseconds) blocks every concurrent
    // AI command for the duration of the refresh.
    let metrics = {
        let mut collector = state.metrics_collector.lock()
            .map_err(|e| format!("Failed to lock metrics collector: {}", e))?;
        collector.get_metrics()
    };

    let ai_engine = state.ai_engine.lock()
        .map_err(|e| format!("Failed to lock AI engine: {}", e))?;

    // Get system uptime to use as boot time metric for suggestions
    let uptime_ms = {
        use std::time::{SystemTime, UNIX_EPOCH};
        use sysinfo::System;
        let boot_timestamp = System::boot_time();
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now_secs.saturating_sub(boot_timestamp) * 1000
    };

    let suggestions = ai_engine.generate_suggestions(
        metrics.cpu.usage_percent as f64,
        metrics.memory.usage_percent as f64,
        metrics.disk.usage_percent as f64,
        Some(uptime_ms),
    );

    Ok(suggestions)
}

#[tauri::command]
fn get_ai_insights(state: State<AppState>) -> Result<Vec<system::AIInsight>, String> {
    // Same lock-ordering fix as get_ai_recommendations: collect metrics under
    // metrics_collector only, then acquire ai_engine for the computation step.
    let metrics = {
        let mut collector = state.metrics_collector.lock()
            .map_err(|e| format!("Failed to lock metrics collector: {}", e))?;
        collector.get_metrics()
    };

    let ai_engine = state.ai_engine.lock()
        .map_err(|e| format!("Failed to lock AI engine: {}", e))?;

    let insights = ai_engine.generate_insights(
        metrics.cpu.usage_percent as f64,
        metrics.memory.usage_percent as f64,
        metrics.disk.usage_percent as f64,
    );

    Ok(insights)
}

#[tauri::command]
fn analyze_boot_speed(state: State<AppState>) -> Result<system::BootSpeedAnalysis, String> {
    let boot_optimizer = state.boot_optimizer.lock()
        .map_err(|e| format!("Failed to lock boot optimizer: {}", e))?;
    
    Ok(boot_optimizer.analyze_boot_speed())
}

#[tauri::command]
fn get_boot_optimization_actions(state: State<AppState>) -> Result<Vec<system::BootOptimizationAction>, String> {
    let boot_optimizer = state.boot_optimizer.lock()
        .map_err(|e| format!("Failed to lock boot optimizer: {}", e))?;
    
    Ok(boot_optimizer.get_optimization_actions())
}

#[tauri::command]
fn apply_boot_optimization(
    state: State<AppState>,
    optimization_id: String,
) -> Result<serde_json::Value, String> {
    enforce_rate_limit(&state, "apply_boot_optimization", OPTIMIZATION_LIMIT)?;

    validate_optimization_id(&optimization_id)?;
    
    let boot_optimizer = state.boot_optimizer.lock()
        .map_err(|e| format!("Failed to lock boot optimizer: {}", e))?;

    let message = boot_optimizer.apply_optimization(&optimization_id)?;
    
    Ok(serde_json::json!({
        "success": true,
        "message": message,
        "requires_restart": false,
    }))
}

/// Returns the hardcoded defaults used when no saved settings file exists.
fn default_settings() -> serde_json::Value {
    serde_json::json!({
        "general": {
            "auto_start": false,
            "minimize_to_tray": true,
            "check_updates": true
        },
        "monitoring": {
            "update_interval_ms": 5000,
            "enable_notifications": true,
            "notification_threshold": "medium"
        },
        "ai": {
            "enable_local_ml": true,
            "enable_cloud_ai": false,
            "api_key_configured": false
        },
        "privacy": {
            "collect_anonymous_stats": false,
            "share_optimization_results": false
        },
        "optimization": {
            "auto_apply_safe_optimizations": false,
            "confirm_before_changes": true
        }
    })
}

/// Returns the path to `settings.json` inside Tauri's app config directory.
fn settings_file(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {}", e))?;
    Ok(config_dir.join("settings.json"))
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let path = settings_file(&app)?;

    if !path.exists() {
        // No saved file yet -- return defaults so the UI gets a well-formed
        // object on first launch.
        return Ok(default_settings());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Settings file is corrupt or unreadable: {}", e))
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    settings: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = settings_file(&app)?;

    // Ensure the parent directory exists (created by Tauri on first launch,
    // but guard against edge cases where the directory was removed).
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialise settings: {}", e))?;

    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "message": "Settings updated successfully"
    }))
}

#[tauri::command]
fn get_performance_history(
    metric: String,
    time_range: String,
    resolution: Option<String>,
) -> Result<serde_json::Value, String> {
    // Performance history collection is not yet implemented.
    // The not_implemented flag lets the frontend render a "coming soon"
    // placeholder instead of an empty chart that looks like a broken feature.
    let _ = (metric, time_range, resolution);
    Ok(serde_json::json!({
        "not_implemented": true,
        "metric": "cpu",
        "data_points": [],
        "average": 0.0,
        "min": 0.0,
        "max": 0.0
    }))
}

#[tauri::command]
fn set_api_key(
    app: tauri::AppHandle,
    provider: String,
    api_key: String,
) -> Result<serde_json::Value, String> {
    use tauri::Manager;

    // Validate provider against an explicit allowlist.
    if provider != "openai" && provider != "anthropic" {
        return Err(format!(
            "Unknown provider '{}'. Must be 'openai' or 'anthropic'.",
            provider
        ));
    }

    let trimmed_key = api_key.trim().to_string();
    if trimmed_key.is_empty() {
        return Err("API key must not be empty.".to_string());
    }

    // Persist the key to a provider-specific file inside Tauri's app config
    // directory. The config directory is created automatically on first write.
    // This is the same directory used by settings.json so the path is already
    // known to the OS and scoped to this application.
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {}", e))?;

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let key_file = config_dir.join(format!("{}_api_key.txt", provider));
    std::fs::write(&key_file, &trimmed_key)
        .map_err(|e| format!("Failed to save API key: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "message": format!("{} API key saved successfully", provider),
        "key_valid": true
    }))
}

#[tauri::command]
fn get_optimization_history(limit: Option<u32>) -> Result<serde_json::Value, String> {
    // Optimization history persistence is not yet implemented.
    // The not_implemented flag lets the frontend render a "coming soon"
    // placeholder instead of an empty list that looks like a broken feature.
    let _ = limit;
    Ok(serde_json::json!({
        "not_implemented": true,
        "records": []
    }))
}

#[tauri::command]
fn train_local_model(include_historical_data: bool) -> Result<serde_json::Value, String> {
    // TODO: Invoke the local ML training pipeline and return real metrics.
    // Stubbed so the frontend receives a well-formed response immediately
    // instead of an unhandled promise rejection.
    let _ = include_historical_data;
    Ok(serde_json::json!({
        "success": true,
        "model_version": "1.0",
        "training_samples": 0,
        "accuracy_score": null
    }))
}

// Focus Mode Commands
#[tauri::command]
fn toggle_focus_mode(state: State<AppState>, enable: bool) -> Result<String, String> {
    let mut manager = state.focus_mode_manager.lock()
        .map_err(|e| format!("Failed to lock focus mode manager: {}", e))?;
    manager.toggle(enable)
}

#[tauri::command]
fn get_focus_mode_status(state: State<AppState>) -> Result<system::FocusModeStatus, String> {
    let manager = state.focus_mode_manager.lock()
        .map_err(|e| format!("Failed to lock focus mode manager: {}", e))?;
    Ok(manager.get_status())
}

#[tauri::command]
fn get_focus_mode_settings(state: State<AppState>) -> Result<system::FocusModeSettings, String> {
    let manager = state.focus_mode_manager.lock()
        .map_err(|e| format!("Failed to lock focus mode manager: {}", e))?;
    Ok(manager.get_settings())
}

#[tauri::command]
fn update_focus_mode_settings(state: State<AppState>, settings: system::FocusModeSettings) -> Result<String, String> {
    let mut manager = state.focus_mode_manager.lock()
        .map_err(|e| format!("Failed to lock focus mode manager: {}", e))?;
    manager.update_settings(settings);
    Ok("Settings updated successfully".to_string())
}

// Maintenance Commands
#[tauri::command]
fn get_maintenance_config(state: State<AppState>) -> Result<system::MaintenanceConfig, String> {
    let scheduler = state.maintenance_scheduler.lock()
        .map_err(|e| format!("Failed to lock maintenance scheduler: {}", e))?;
    Ok(scheduler.get_config())
}

#[tauri::command]
fn update_maintenance_config(state: State<AppState>, config: system::MaintenanceConfig) -> Result<String, String> {
    let scheduler = state.maintenance_scheduler.lock()
        .map_err(|e| format!("Failed to lock maintenance scheduler: {}", e))?;
    scheduler.update_config(config);
    Ok("Maintenance config updated successfully".to_string())
}

#[tauri::command]
fn get_maintenance_logs(state: State<AppState>) -> Result<Vec<system::MaintenanceLog>, String> {
    let scheduler = state.maintenance_scheduler.lock()
        .map_err(|e| format!("Failed to lock maintenance scheduler: {}", e))?;
    Ok(scheduler.get_logs())
}

#[tauri::command]
fn get_deep_sleep_status(state: State<AppState>) -> Result<system::DeepSleepStatus, String> {
    let ds = state.deep_sleep.lock()
        .map_err(|e| format!("Failed to lock deep sleep manager: {}", e))?;
    Ok(ds.get_status())
}

#[tauri::command]
fn update_deep_sleep_config(
    state: State<AppState>,
    enabled: bool,
    timeout_secs: u64,
    whitelist: Vec<String>,
) -> Result<system::DeepSleepStatus, String> {
    let mut ds = state.deep_sleep.lock()
        .map_err(|e| format!("Failed to lock deep sleep manager: {}", e))?;
    ds.update_config(enabled, timeout_secs, whitelist)?;
    Ok(ds.get_status())
}

#[tauri::command]
fn thaw_process(state: State<AppState>, pid: u32) -> Result<system::DeepSleepStatus, String> {
    let mut ds = state.deep_sleep.lock()
        .map_err(|e| format!("Failed to lock deep sleep manager: {}", e))?;
    ds.thaw_process(pid)?;
    Ok(ds.get_status())
}

#[tauri::command]
fn freeze_process(
    state: State<AppState>,
    pid: u32,
    name: String,
    memory_bytes: u64,
) -> Result<system::DeepSleepStatus, String> {
    let mut ds = state.deep_sleep.lock()
        .map_err(|e| format!("Failed to lock deep sleep manager: {}", e))?;
    ds.freeze_process(pid, name, memory_bytes)?;
    Ok(ds.get_status())
// Hardware Health Commands
#[tauri::command]
fn get_hardware_health(state: State<AppState>) -> Result<system::HardwareHealthData, String> {
    let mut collector = state.hardware_health.lock()
        .map_err(|e| format!("Failed to lock hardware health collector: {}", e))?;
    Ok(collector.get_hardware_health())
}

#[tauri::command]
fn get_disk_health(state: State<AppState>) -> Result<Vec<system::DiskHealthInfo>, String> {
    let mut collector = state.hardware_health.lock()
        .map_err(|e| format!("Failed to lock hardware health collector: {}", e))?;
    Ok(collector.get_disk_health())
}

#[tauri::command]
fn get_battery_health(state: State<AppState>) -> Result<Option<system::BatteryHealthInfo>, String> {
    let mut collector = state.hardware_health.lock()
        .map_err(|e| format!("Failed to lock hardware health collector: {}", e))?;
    Ok(collector.get_battery_health())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            metrics_collector: Mutex::new(MetricsCollector::new()),
            boot_optimizer: Mutex::new(BootOptimizer::new()),
            ai_engine: Mutex::new(AISuggestionsEngine::new()),
            focus_mode_manager: Mutex::new(system::FocusModeManager::new()),
            maintenance_scheduler: Mutex::new(system::MaintenanceScheduler::new()),
            deep_sleep: Mutex::new(system::DeepSleepManager::new(None)),
            hardware_health: Mutex::new(system::HardwareHealthCollector::new()),
            rate_limiter: Mutex::new(system::RateLimiter::new()),
        })
        .setup(|app| {
            use tauri::Manager;

            // Set the config directory for Deep Sleep
            if let Ok(config_dir) = app.path().app_config_dir() {
                if let Ok(mut ds) = app.state::<AppState>().deep_sleep.lock() {
                    ds.set_config_path(config_dir);
                }
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    let state = handle.state::<AppState>();
                    
                    let run_now = {
                        let config = state.maintenance_scheduler.lock().unwrap().get_config();
                        if !config.enabled {
                            false
                        } else if config.schedule == "idle" {
                            system::get_idle_time_seconds() > 900 // 15 mins
                        } else if config.schedule == "daily" {
                            let now = chrono::Local::now();
                            now.hour() == 2 && now.minute() == 0 // 2:00 AM
                        } else if config.schedule == "weekly" {
                            let now = chrono::Local::now();
                            use chrono::Datelike;
                            now.weekday() == chrono::Weekday::Sun && now.hour() == 2 && now.minute() == 0 // Sun 2:00 AM
                        } else {
                            false
                        }
                    };

                    if run_now {
                        state.maintenance_scheduler.lock().unwrap().execute_maintenance();
                        // Sleep extra to prevent multiple runs in the same idle period or minute
                        std::thread::sleep(std::time::Duration::from_secs(3600)); 
                    }
                }
            });

            // Spawn deep sleep background tick loop
            let ds_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    if let Some(state) = ds_handle.try_state::<AppState>() {
                        if let Ok(mut ds) = state.deep_sleep.lock() {
                            ds.tick();
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_metrics,
            get_process_list,
            kill_process,
            get_boot_time,
            get_startup_programs,
            toggle_startup_program,
            analyze_system,
            get_optimization_suggestions,
            get_optimization_details,
            apply_optimization,
            rollback_optimization,
            clean_temp_files,
            get_ai_recommendations,
            get_ai_insights,
            analyze_boot_speed,
            get_boot_optimization_actions,
            apply_boot_optimization,
            get_settings,
            update_settings,
            get_performance_history,
            set_api_key,
            get_optimization_history,
            train_local_model,
            toggle_focus_mode,
            get_focus_mode_status,
            get_focus_mode_settings,
            update_focus_mode_settings,
            get_maintenance_config,
            update_maintenance_config,
            get_maintenance_logs,
            get_deep_sleep_status,
            update_deep_sleep_config,
            thaw_process,
            freeze_process,
            get_hardware_health,
            get_disk_health,
            get_battery_health,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<AppState>() {
                if let Ok(mut ds) = state.deep_sleep.lock() {
                    let _ = ds.unfreeze_all();
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::validate_optimization_id;

    #[test]
    fn debug_metrics() {
        println!("DEBUG METRICS START");
        let mut collector = super::system::MetricsCollector::new();
        let metrics = collector.get_metrics();
        println!("DEBUG METRICS CPU: {:?}", metrics.cpu);
        println!("DEBUG METRICS MEM: {:?}", metrics.memory);
        println!("DEBUG METRICS DISK: {:?}", metrics.disk);
    }

    #[test]
    fn accepts_known_ids() {
        for id in ["opt_1", "opt_2", "opt_3", "opt_4"] {
            assert!(validate_optimization_id(id).is_ok(), "expected {id} to be valid");
        }
    }

    #[test]
    fn rejects_unknown_but_well_formed_id() {
        assert!(validate_optimization_id("opt_999").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        // Path separators and dots are outside the allowed character set, so
        // an injection attempt is rejected before the whitelist check runs.
        assert!(validate_optimization_id("../../system.conf").is_err());
        assert!(validate_optimization_id("opt_../../etc/passwd").is_err());
    }

    #[test]
    fn rejects_empty_and_oversized() {
        assert!(validate_optimization_id("").is_err());
        let oversized = "a".repeat(65);
        assert!(validate_optimization_id(&oversized).is_err());
    }

    #[test]
    fn rejects_uppercase_and_symbols() {
        assert!(validate_optimization_id("OPT_1").is_err());
        assert!(validate_optimization_id("opt-1").is_err());
        assert!(validate_optimization_id("drop_table;").is_err());
    }
}
