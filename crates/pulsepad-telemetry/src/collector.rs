use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub packet_rate: f64,
    pub dropped_packets: u64,
    pub total_packets: u64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub connection_uptime_secs: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug, Clone)]
struct LatencyEntry {
    timestamp: Instant,
    latency_us: u64,
}

#[derive(Debug)]
struct PacketStats {
    total_sent: u64,
    total_received: u64,
    total_dropped: u64,
    last_reset: Instant,
}

#[derive(Debug)]
pub struct TelemetryCollector {
    packet_stats: Arc<RwLock<PacketStats>>,
    latencies: Arc<RwLock<VecDeque<LatencyEntry>>>,
    start_time: Instant,
    max_latency_history: usize,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self {
            packet_stats: Arc::new(RwLock::new(PacketStats {
                total_sent: 0,
                total_received: 0,
                total_dropped: 0,
                last_reset: Instant::now(),
            })),
            latencies: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            start_time: Instant::now(),
            max_latency_history: 1000,
        }
    }

    pub fn record_packet_sent(&self) {
        self.packet_stats.write().total_sent += 1;
    }

    pub fn record_packet_received(&self) {
        self.packet_stats.write().total_received += 1;
    }

    pub fn record_packet_dropped(&self) {
        self.packet_stats.write().total_dropped += 1;
    }

    pub fn record_latency(&self, latency_us: u64) {
        let mut latencies = self.latencies.write();
        latencies.push_back(LatencyEntry {
            timestamp: Instant::now(),
            latency_us,
        });

        // Keep only recent entries
        while latencies.len() > self.max_latency_history {
            latencies.pop_front();
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let stats = self.packet_stats.read();
        let latencies = self.latencies.read();

        let now = Instant::now();
        let elapsed_secs = now.duration_since(stats.last_reset).as_secs_f64();
        let uptime_secs = now.duration_since(self.start_time).as_secs();

        let packet_rate = if elapsed_secs > 0.0 {
            stats.total_received as f64 / elapsed_secs
        } else {
            0.0
        };

        let (avg_latency, min_latency, max_latency) = if latencies.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let total: u64 = latencies.iter().map(|e| e.latency_us).sum();
            let min = latencies.iter().map(|e| e.latency_us).min().unwrap_or(0);
            let max = latencies.iter().map(|e| e.latency_us).max().unwrap_or(0);
            (
                total as f64 / latencies.len() as f64 / 1000.0,
                min as f64 / 1000.0,
                max as f64 / 1000.0,
            )
        };

        // Get system memory usage
        let memory_usage_mb = Self::get_memory_usage();

        // Get CPU usage (simplified)
        let cpu_usage_percent = Self::get_cpu_usage();

        MetricsSnapshot {
            timestamp: Utc::now(),
            packet_rate,
            dropped_packets: stats.total_dropped,
            total_packets: stats.total_received,
            avg_latency_ms: avg_latency,
            min_latency_ms: min_latency,
            max_latency_ms: max_latency,
            connection_uptime_secs: uptime_secs,
            memory_usage_mb,
            cpu_usage_percent,
        }
    }

    fn get_memory_usage() -> f64 {
        // Platform-specific memory usage
        // This is a placeholder - implement with platform APIs
        #[cfg(target_os = "macos")]
        {
            // Use mach_task_basic_info or similar
            0.0
        }
        #[cfg(target_os = "windows")]
        {
            // Use GetProcessMemoryInfo
            0.0
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            0.0
        }
    }

    fn get_cpu_usage() -> f64 {
        // Platform-specific CPU usage
        // This is a placeholder - implement with platform APIs
        0.0
    }

    pub fn reset(&self) {
        let mut stats = self.packet_stats.write();
        stats.total_sent = 0;
        stats.total_received = 0;
        stats.total_dropped = 0;
        stats.last_reset = Instant::now();

        self.latencies.write().clear();
    }

    pub fn get_packet_rate(&self) -> f64 {
        let stats = self.packet_stats.read();
        let elapsed = stats.last_reset.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            stats.total_received as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn get_dropped_packets(&self) -> u64 {
        self.packet_stats.read().total_dropped
    }

    pub fn get_avg_latency_ms(&self) -> f64 {
        let latencies = self.latencies.read();
        if latencies.is_empty() {
            0.0
        } else {
            let total: u64 = latencies.iter().map(|e| e.latency_us).sum();
            total as f64 / latencies.len() as f64 / 1000.0
        }
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}
