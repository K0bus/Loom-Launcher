pub mod collector;
pub mod commands;
pub mod models;

pub use collector::{SharedCollector, SystemMetricsCollector};
pub use commands::*;
pub use models::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_initialization_and_snapshot() {
        let mut collector = SystemMetricsCollector::new();
        let snapshot = collector.collect_snapshot();

        assert!(snapshot.timestamp_ms > 0);
        assert!(!snapshot.cpu.model.is_empty());
        assert!(snapshot.cpu.logical_cores > 0);
        assert!(snapshot.memory.total_bytes > 0);
        assert!(!snapshot.os.os_name.is_empty());
        assert!(!snapshot.os.uptime_formatted.is_empty());
    }

    #[test]
    fn test_system_summary() {
        let mut collector = SystemMetricsCollector::new();
        let summary = collector.get_system_summary();

        assert!(!summary.os_name.is_empty());
        assert!(!summary.cpu_model.is_empty());
        assert!(summary.logical_cores > 0);
        assert!(summary.total_memory_bytes > 0);
    }

    #[test]
    fn test_kill_process_safety_guards() {
        // PID 0 must be rejected
        let res0 = kill_process(0, false);
        assert!(res0.is_err());
        assert!(res0.unwrap_err().contains("Action interdite"));

        // PID 1 (init/systemd) must be rejected
        let res1 = kill_process(1, false);
        assert!(res1.is_err());
        assert!(res1.unwrap_err().contains("Action interdite"));

        // Own PID must be rejected
        let own_pid = std::process::id();
        let res_own = kill_process(own_pid, false);
        assert!(res_own.is_err());
        assert!(res_own.unwrap_err().contains("Action interdite"));
    }
}
