//! Alert rules and threshold evaluation engine.
//!
//! Stores alert rules in memory. Rules define thresholds on health metrics
//! (buffer utilization, push errors, ClickHouse flush errors, etc.) and
//! trigger when conditions are met. Fired alerts are stored in a bounded
//! history ring buffer.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of fired alert entries to keep in history.
const MAX_HISTORY: usize = 500;

/// A metric that can be monitored by an alert rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertMetric {
    BufferUtilization,
    PushErrorRate,
    PushErrorsTotal,
    ClickhouseFlushErrors,
    ClickhousePendingRows,
    EventThroughput,
}

impl AlertMetric {
    pub fn label(&self) -> &'static str {
        match self {
            Self::BufferUtilization => "Buffer Utilization",
            Self::PushErrorRate => "Push Error Rate",
            Self::PushErrorsTotal => "Push Errors (total)",
            Self::ClickhouseFlushErrors => "ClickHouse Flush Errors",
            Self::ClickhousePendingRows => "ClickHouse Pending Rows",
            Self::EventThroughput => "Event Throughput",
        }
    }
}

/// Comparison operator for threshold evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
}

impl Operator {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Gt => value > threshold,
            Self::Gte => value >= threshold,
            Self::Lt => value < threshold,
            Self::Lte => value <= threshold,
            Self::Eq => (value - threshold).abs() < f64::EPSILON,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Eq => "==",
        }
    }
}

/// Severity level for an alert rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// An alert rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub metric: AlertMetric,
    pub operator: Operator,
    pub threshold: f64,
    pub severity: Severity,
    pub enabled: bool,
    pub created_at: u64,
}

/// A fired alert entry (when a rule's condition was met).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: Severity,
    pub metric: AlertMetric,
    pub value: f64,
    pub threshold: f64,
    pub operator: Operator,
    pub message: String,
    pub fired_at: u64,
}

/// A snapshot of current metric values used for evaluation.
#[derive(Debug, Clone, Default)]
pub struct MetricSnapshot {
    pub buffer_utilization: f64,
    pub push_error_rate: f64,
    pub push_errors_total: f64,
    pub clickhouse_flush_errors: f64,
    pub clickhouse_pending_rows: f64,
    pub event_throughput: f64,
}

impl MetricSnapshot {
    pub fn get(&self, metric: AlertMetric) -> f64 {
        match metric {
            AlertMetric::BufferUtilization => self.buffer_utilization,
            AlertMetric::PushErrorRate => self.push_error_rate,
            AlertMetric::PushErrorsTotal => self.push_errors_total,
            AlertMetric::ClickhouseFlushErrors => self.clickhouse_flush_errors,
            AlertMetric::ClickhousePendingRows => self.clickhouse_pending_rows,
            AlertMetric::EventThroughput => self.event_throughput,
        }
    }
}

/// Thread-safe alert store. Manages rules and evaluation history.
pub struct AlertStore {
    inner: Mutex<AlertStoreInner>,
}

struct AlertStoreInner {
    rules: Vec<AlertRule>,
    history: VecDeque<AlertEvent>,
    next_id: u64,
}

impl Default for AlertStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AlertStoreInner {
                rules: Vec::new(),
                history: VecDeque::new(),
                next_id: 1,
            }),
        }
    }

    /// Create a new alert rule. Returns the assigned ID.
    pub fn create_rule(
        &self,
        name: String,
        metric: AlertMetric,
        operator: Operator,
        threshold: f64,
        severity: Severity,
    ) -> String {
        let mut inner = self.inner.lock().unwrap();
        let id = format!("alert-{}", inner.next_id);
        inner.next_id += 1;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        inner.rules.push(AlertRule {
            id: id.clone(),
            name,
            metric,
            operator,
            threshold,
            severity,
            enabled: true,
            created_at: now,
        });
        id
    }

    /// Delete a rule by ID. Returns true if found and removed.
    pub fn delete_rule(&self, id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let before = inner.rules.len();
        inner.rules.retain(|r| r.id != id);
        inner.rules.len() < before
    }

    /// Toggle a rule's enabled state. Returns the new state, or None if not found.
    pub fn toggle_rule(&self, id: &str) -> Option<bool> {
        let mut inner = self.inner.lock().unwrap();
        for rule in &mut inner.rules {
            if rule.id == id {
                rule.enabled = !rule.enabled;
                return Some(rule.enabled);
            }
        }
        None
    }

    /// List all rules.
    pub fn list_rules(&self) -> Vec<AlertRule> {
        self.inner.lock().unwrap().rules.clone()
    }

    /// List recent fired alert events.
    pub fn list_history(&self, limit: usize) -> Vec<AlertEvent> {
        let inner = self.inner.lock().unwrap();
        inner.history.iter().rev().take(limit).cloned().collect()
    }

    /// Evaluate all enabled rules against the current metrics snapshot.
    /// Returns newly fired alerts.
    pub fn evaluate(&self, snapshot: &MetricSnapshot) -> Vec<AlertEvent> {
        let mut inner = self.inner.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let fired: Vec<AlertEvent> = inner
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .filter_map(|rule| {
                let value = snapshot.get(rule.metric);
                if rule.operator.evaluate(value, rule.threshold) {
                    Some(AlertEvent {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        severity: rule.severity,
                        metric: rule.metric,
                        value,
                        threshold: rule.threshold,
                        operator: rule.operator,
                        message: format!(
                            "{}: {} {} {} (current: {:.2})",
                            rule.name,
                            rule.metric.label(),
                            rule.operator.symbol(),
                            rule.threshold,
                            value,
                        ),
                        fired_at: now,
                    })
                } else {
                    None
                }
            })
            .collect();

        for event in &fired {
            inner.history.push_back(event.clone());
        }

        // Trim history
        while inner.history.len() > MAX_HISTORY {
            inner.history.pop_front();
        }

        fired
    }

    /// Count of active (enabled) rules.
    pub fn active_rule_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .rules
            .iter()
            .filter(|r| r.enabled)
            .count()
    }

    /// Count of fired events in history.
    pub fn history_count(&self) -> usize {
        self.inner.lock().unwrap().history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_list_rules() {
        let store = AlertStore::new();
        let id = store.create_rule(
            "High buffer".into(),
            AlertMetric::BufferUtilization,
            Operator::Gt,
            0.9,
            Severity::Warning,
        );
        assert!(id.starts_with("alert-"));
        let rules = store.list_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "High buffer");
    }

    #[test]
    fn test_delete_rule() {
        let store = AlertStore::new();
        let id = store.create_rule(
            "test".into(),
            AlertMetric::PushErrorsTotal,
            Operator::Gt,
            100.0,
            Severity::Critical,
        );
        assert!(store.delete_rule(&id));
        assert!(!store.delete_rule(&id));
        assert_eq!(store.list_rules().len(), 0);
    }

    #[test]
    fn test_toggle_rule() {
        let store = AlertStore::new();
        let id = store.create_rule(
            "test".into(),
            AlertMetric::BufferUtilization,
            Operator::Gte,
            0.8,
            Severity::Info,
        );
        assert_eq!(store.toggle_rule(&id), Some(false));
        assert_eq!(store.toggle_rule(&id), Some(true));
        assert_eq!(store.toggle_rule("nonexistent"), None);
    }

    #[test]
    fn test_evaluate_fires_alert() {
        let store = AlertStore::new();
        store.create_rule(
            "Buffer high".into(),
            AlertMetric::BufferUtilization,
            Operator::Gt,
            0.8,
            Severity::Warning,
        );
        let snapshot = MetricSnapshot {
            buffer_utilization: 0.95,
            ..Default::default()
        };
        let fired = store.evaluate(&snapshot);
        assert_eq!(fired.len(), 1);
        assert!(fired[0].message.contains("Buffer high"));
        assert_eq!(store.history_count(), 1);
    }

    #[test]
    fn test_evaluate_does_not_fire_below_threshold() {
        let store = AlertStore::new();
        store.create_rule(
            "Buffer high".into(),
            AlertMetric::BufferUtilization,
            Operator::Gt,
            0.8,
            Severity::Warning,
        );
        let snapshot = MetricSnapshot {
            buffer_utilization: 0.5,
            ..Default::default()
        };
        let fired = store.evaluate(&snapshot);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_disabled_rule_does_not_fire() {
        let store = AlertStore::new();
        let id = store.create_rule(
            "Buffer high".into(),
            AlertMetric::BufferUtilization,
            Operator::Gt,
            0.8,
            Severity::Warning,
        );
        store.toggle_rule(&id);
        let snapshot = MetricSnapshot {
            buffer_utilization: 0.95,
            ..Default::default()
        };
        let fired = store.evaluate(&snapshot);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_operator_evaluate() {
        assert!(Operator::Gt.evaluate(10.0, 5.0));
        assert!(!Operator::Gt.evaluate(5.0, 10.0));
        assert!(Operator::Gte.evaluate(5.0, 5.0));
        assert!(Operator::Lt.evaluate(3.0, 5.0));
        assert!(Operator::Lte.evaluate(5.0, 5.0));
        assert!(Operator::Eq.evaluate(5.0, 5.0));
    }
}
