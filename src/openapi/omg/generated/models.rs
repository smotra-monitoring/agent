//!
//! Generated from an OAS specification by openapi-model-generator(v0.5.1)
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// AgentStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Version of the agent
    pub agent_version: String,
    /// Version of the agent configuration
    pub config_version: i64,
    /// Whether the agent is currently running
    pub is_running: bool,
    /// Timestamp when the agent started in UTC (RFC3339), null if never started
    pub started_at: DateTime<Utc>,
    /// Timestamp when the agent stopped in UTC (RFC3339), null if running
    pub stopped_at: Option<DateTime<Utc>>,
    /// Total number of checks performed by the agent
    pub checks_performed: i64,
    /// Number of successful checks
    pub checks_successful: i64,
    /// Number of failed checks
    pub checks_failed: i64,
    /// Timestamp of the last report received from the agent (RFC3339)
    pub last_report_at: DateTime<Utc>,
    /// Number of consecutive failed report attempts
    pub failed_report_count: i64,
    /// Whether the agent is currently connected to the server
    pub server_connected: bool,
    pub cache_stats: AgentCacheStats,
}

/// AgentCacheStats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCacheStats {
    /// Number of results currently buffered in the local cache
    pub len: i64,
    /// Maximum number of results the cache can hold (hard cap)
    pub capacity: i64,
}

/// AgentConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Configuration version (used for syncing with server)
    pub version: i64,
    pub agent_id: UUIDv7,
    /// Human-readable agent name
    pub agent_name: String,
    /// Tags for this agent (used for mesh organization)
    pub tags: Option<Vec<String>>,
    pub monitoring: MonitoringConfig,
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub self_upgrade: SelfUpgradeConfig,
    /// Endpoints to monitor
    pub endpoints: Vec<Endpoint>,
}

/// MonitoringConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Interval between checks in seconds
    pub interval_secs: i64,
    /// Timeout for each check in seconds
    pub timeout_secs: i64,
    /// Number of pings to send per check
    pub ping_count: i64,
    /// Maximum number of concurrent checks
    pub max_concurrent: i64,
    /// Enable traceroute on failed pings
    pub traceroute_on_failure: bool,
    /// Maximum TTL for traceroute
    pub traceroute_max_hops: i64,
}

/// ServerConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server URL
    pub url: Option<String>,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Report interval in seconds
    pub report_interval_secs: i64,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: i64,
    /// Enable TLS verification
    pub verify_tls: bool,
    /// Connection timeout in seconds
    pub timeout_secs: i64,
    /// Retry attempts on failure
    pub retry_attempts: i64,
}

/// StorageConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory for storing cached data
    pub cache_dir: String,
    /// Maximum number of results to cache
    pub max_cached_results: i64,
    /// Maximum age of cached results in seconds (e.g., 86400 = 24 hours)
    pub max_cache_age_secs: i64,
}

/// SelfUpgradeConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfUpgradeConfig {
    /// Enables self-upgrade checks when true
    pub enabled: bool,
    /// GitHub repository URL for checking latest releases (must be a public repo)
    pub github_repo_url: String,
    /// Periodic self-upgrade check interval in seconds
    pub check_interval_secs: i64,
}

/// An endpoint to monitor (IP address, hostname, or URL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: UUIDv7,
    /// IP address, hostname, or URL
    pub address: String,
    pub port: Option<i64>,
    pub enabled: bool,
    /// Tags associated with the target
    pub tags: Vec<String>,
}

/// MonitoringResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringResult {
    pub id: UUIDv7,
    pub agent_id: UUIDv7,
    pub endpoint_id: UUIDv7,
    pub check_type: CheckType,
    /// Timestamp when the report was generated (RFC3339)
    pub timestamp: DateTime<Utc>,
}

/// A batch of monitoring results submitted by an agent from its local cache.
/// The server deduplicates entries by `MonitoringResult.id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMonitoringResults {
    /// Ordered list of monitoring results (oldest-first)
    pub results: Vec<MonitoringResult>,
}

/// CheckType (oneOf)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CheckType {
    PingCheck(PingCheck),
    TracerouteCheck(TracerouteCheck),
    TcpConnectCheck(TcpConnectCheck),
    UdpConnectCheck(UdpConnectCheck),
    HttpGetCheck(HttpGetCheck),
    PluginCheck(PluginCheck),
}
/// PingCheckType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PingCheckType {
    #[serde(rename = "ping")]
    Ping,
}
/// TracerouteCheckType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TracerouteCheckType {
    #[serde(rename = "traceroute")]
    Traceroute,
}
/// TcpConnectCheckType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TcpConnectCheckType {
    #[serde(rename = "tcpconnect")]
    Tcpconnect,
}
/// UdpConnectCheckType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UdpConnectCheckType {
    #[serde(rename = "udpconnect")]
    Udpconnect,
}
/// HttpGetCheckType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpGetCheckType {
    #[serde(rename = "httpget")]
    Httpget,
}
/// PluginCheckType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginCheckType {
    #[serde(rename = "plugin")]
    Plugin,
}
/// PingCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingCheck {
    #[serde(rename = "type")]
    pub r#type: PingCheckType,
    pub result: PingResult,
}

/// PingResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    /// Resolved IP address of the target
    pub resolved_ip: String,
    pub successes: i64,
    pub failures: i64,
    pub success_latencies: Vec<f64>,
    pub error_details: Option<ErrorDetails>,
}

/// TracerouteCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteCheck {
    #[serde(rename = "type")]
    pub r#type: TracerouteCheckType,
    pub result: TracerouteResult,
}

/// TracerouteResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteResult {
    pub hops: Vec<TracerouteHop>,
    pub target_reached: bool,
    pub error_details: Option<ErrorDetails>,
}

/// TracerouteHop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteHop {
    pub hop: i64,
    /// Resolved IP address of the target
    pub resolved_ip: Option<String>,
    pub success_latencies: Option<Vec<f64>>,
    pub hostname: Option<String>,
}

/// TcpConnectCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnectCheck {
    #[serde(rename = "type")]
    pub r#type: TcpConnectCheckType,
    pub result: TcpConnectResult,
}

/// TcpConnectResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnectResult {
    pub connected: bool,
    pub connect_time_ms: Option<f64>,
    pub error_details: Option<ErrorDetails>,
    pub resolved_ip: String,
}

/// UdpConnectCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConnectCheck {
    #[serde(rename = "type")]
    pub r#type: UdpConnectCheckType,
    pub result: UdpConnectResult,
}

/// UdpConnectResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConnectResult {
    pub probe_successful: bool,
    pub response_time_ms: Option<f64>,
    pub error_details: Option<ErrorDetails>,
    pub resolved_ip: String,
}

/// HttpGetCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGetCheck {
    #[serde(rename = "type")]
    pub r#type: HttpGetCheckType,
    pub result: HttpGetResult,
}

/// HttpGetResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGetResult {
    pub status_code: i64,
    pub response_time_ms: Option<f64>,
    pub response_size_bytes: Option<i64>,
    pub error_details: Option<ErrorDetails>,
    pub success: bool,
}

/// PluginCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCheck {
    #[serde(rename = "type")]
    pub r#type: PluginCheckType,
    pub result: PluginResult,
}

/// Data
pub type Data = std::collections::HashMap<String, String>;

/// PluginResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub plugin_name: String,
    pub plugin_version: String,
    pub success: bool,
    pub response_time_ms: Option<f64>,
    pub error_details: Option<ErrorDetails>,
    pub data: std::collections::HashMap<String, String>,
}

/// Error information from a check, stored as an extensible JSON object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// List of error messages from the check
    pub errors: Option<Vec<String>>,
}

/// AgentHeartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    /// Agent-local timestamp when the heartbeat was generated (RFC3339)
    pub timestamp: DateTime<Utc>,
    pub health_status: AgentHealthStatus,
    pub metrics: AgentMetrics,
    pub agent_status: AgentStatus,
}

/// AgentMetrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// How long the agent process has been running in seconds; useful for detecting crashes and restarts
    pub agent_uptime_secs: i64,
    /// CPU utilization percentage (0.0–100.0)
    pub cpu_usage_percent: f64,
    /// Resident memory currently in use (MB)
    pub memory_usage_mb: f64,
    /// Total physical memory available (MB)
    pub memory_total_mb: f64,
    /// System uptime in seconds
    pub system_uptime_secs: i64,
}

/// Type of check performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "icmp_ping")]
    IcmpPing,
    #[serde(rename = "tcp_check")]
    TcpCheck,
    #[serde(rename = "http_check")]
    HttpCheck,
    #[serde(rename = "traceroute")]
    Traceroute,
    #[serde(rename = "custom")]
    Custom,
}
/// Additional metric-specific data
pub type Metadata = std::collections::HashMap<String, serde_json::Value>;

/// Metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Type of check performed
    #[serde(rename = "type")]
    pub r#type: Type,
    /// Target host, IP, or URL
    pub target: String,
    pub status: MetricStatus,
    /// Response time in milliseconds
    pub response_time_ms: Option<f64>,
    /// Packet loss percentage for ping checks
    pub packet_loss_percent: Option<f64>,
    /// HTTP status code for HTTP checks
    pub status_code: Option<i64>,
    /// Error message if check failed
    pub error_message: Option<String>,
    /// Additional metric-specific data
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Custom tags for organizing agents
pub type Tags = std::collections::HashMap<String, String>;

/// AgentRegistration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub hostname: String,
    pub ip_address: String,
    pub agent_version: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    /// Custom tags for organizing agents
    pub tags: Option<std::collections::HashMap<String, String>>,
}

/// AgentNetworkInterface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNetworkInterface {
    /// IP address of the network interface (IPv4 or IPv6, excluding loopback and link-local)
    pub ip: String,
    /// Name of the network interface
    pub iface: String,
    pub family: IpAddressFamily,
    /// Whether this address is recommended for the server to use when communicating
    /// with the agent. Determined by the agent using the OS routing table: the source
    /// IP the OS selects when opening a connection toward the server is marked as
    /// recommended. Only one entry will have recommended=true.
    pub recommended: bool,
}

/// Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: UUIDv7,
    /// Section (organizational unit) this agent belongs to
    #[serde(rename = "sectionId")]
    pub section_id: String,
    /// Human-readable agent name
    pub name: String,
    /// Current configuration version
    #[serde(rename = "configVersion")]
    pub config_version: i64,
    /// Version of the agent software, null if not yet reported
    #[serde(rename = "agentVersion")]
    pub agent_version: Option<String>,
    /// Network interfaces reported by the agent
    #[serde(rename = "ipAddresses")]
    pub ip_addresses: Option<Vec<AgentNetworkInterface>>,
    /// Timestamp of the last heartbeat received from the agent
    #[serde(rename = "lastSeenAt")]
    pub last_seen_at: Option<DateTime<Utc>>,
    /// Timestamp of the last monitoring result submission
    #[serde(rename = "lastResultSubmittedAt")]
    pub last_result_submitted_at: Option<DateTime<Utc>>,
    /// Timestamp when the agent was registered
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    /// Timestamp when the agent record was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

/// AgentListResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<Agent>,
    pub pagination: Pagination,
}

/// AgentSelfRegistration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSelfRegistration {
    #[serde(rename = "agentId")]
    pub agent_id: UUIDv7,
    /// SHA-256 hash of the claim token (plain token shown in agent logs for user)
    #[serde(rename = "claimTokenHash")]
    pub claim_token_hash: String,
    /// System hostname of the machine running the agent
    pub hostname: String,
    /// Version of the agent software
    #[serde(rename = "agentVersion")]
    pub agent_version: String,
    /// List of all non-loopback, non-link-local network interfaces on the agent host.
    /// Loopback (127.x.x.x / ::1) and link-local (169.254.x.x / fe80::/10) addresses
    /// are excluded. The server should store all addresses and allow the operator to
    /// select the preferred one during the claim process. The entry with
    /// recommended=true reflects the OS-selected source IP for connections toward
    /// the server (determined via routing table, no traffic sent).
    #[serde(rename = "ipAddresses")]
    pub ip_addresses: Vec<AgentNetworkInterface>,
}

/// AgentRegistrationResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResponse {
    pub status: RegistrationStatus,
    /// URL for agent to poll for claim status
    #[serde(rename = "pollUrl")]
    pub poll_url: String,
    /// URL for user to claim the agent (web UI)
    #[serde(rename = "claimUrl")]
    pub claim_url: String,
    /// When the claim token expires (RFC3339)
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
}

/// ClaimStatusPending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimStatusPending {
    pub status: ClaimStatusPendingEnum,
    /// When the claim token expires (RFC3339)
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    /// Seconds until next poll (server-controlled backoff)
    #[serde(rename = "pollIn")]
    pub poll_in: i64,
}

/// ClaimStatusClaimed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimStatusClaimed {
    pub status: ClaimStatusClaimedEnum,
    /// API key for authenticated requests (one-time delivery)
    #[serde(rename = "apiKey")]
    pub api_key: String,
    /// URL to fetch agent configuration
    #[serde(rename = "configUrl")]
    pub config_url: String,
}

/// ClaimAgentRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAgentRequest {
    #[serde(rename = "agentId")]
    pub agent_id: UUIDv7,
    /// Claim token from agent logs
    #[serde(rename = "claimToken")]
    pub claim_token: String,
    #[serde(rename = "sectionId")]
    pub section_id: UUIDv7,
    /// Human-readable name for the agent (defaults to hostname if not provided)
    pub name: Option<String>,
}

/// ClaimAgentResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAgentResponse {
    #[serde(rename = "agentId")]
    pub agent_id: UUIDv7,
    pub status: ClaimResponseStatus,
    /// Human-readable message
    pub message: String,
}

/// Configuration
pub type Configuration = std::collections::HashMap<String, serde_json::Value>;

/// PluginConfiguration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfiguration {
    pub name: String,
    pub version: Option<String>,
    pub enabled: bool,
    pub configuration: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub response_time_warning_ms: Option<f64>,
    pub response_time_critical_ms: Option<f64>,
    pub packet_loss_warning_percent: Option<f64>,
    pub packet_loss_critical_percent: Option<f64>,
}

/// RetryPolicy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: Option<i64>,
    pub retry_delay_seconds: Option<i64>,
    pub backoff_multiplier: Option<f64>,
}

/// ResultsBatchAcknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsBatchAcknowledgment {
    pub submission_id: UUIDv7,
    /// Number of results accepted for processing
    pub accepted: i64,
    /// Number of results deduplicated (already known to the server)
    pub duplicates_skipped: Option<i64>,
    /// Timestamp when the batch was received by the server
    pub received_at: DateTime<Utc>,
}

/// TimeRange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// ResultReport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultReport {
    pub time_range: TimeRange,
    pub aggregation: Option<String>,
    pub data: Vec<AggregatedMetric>,
    pub pagination: Pagination,
    pub metadata: Option<Metadata>,
}

/// AggregatedMetric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub timestamp: DateTime<Utc>,
    pub agent_id: UUIDv7,
    pub hostname: Option<String>,
    pub target: String,
    pub metric_type: String,
    pub status: Option<AggregatedMetricStatus>,
    pub avg_response_time_ms: Option<f64>,
    pub min_response_time_ms: Option<f64>,
    pub max_response_time_ms: Option<f64>,
    pub p50_response_time_ms: Option<f64>,
    pub p95_response_time_ms: Option<f64>,
    pub p99_response_time_ms: Option<f64>,
    pub success_rate_percent: Option<f64>,
    pub check_count: Option<i64>,
}

/// MetricsResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub metrics: Vec<serde_json::Value>,
    pub pagination: Pagination,
}

/// SummaryStatistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryStatistics {
    pub time_range: Option<String>,
    pub total_agents: Option<i64>,
    pub active_agents: Option<i64>,
    pub inactive_agents: Option<i64>,
    pub total_targets: Option<i64>,
    pub reachable_targets: Option<i64>,
    pub unreachable_targets: Option<i64>,
    pub degraded_targets: Option<i64>,
    pub total_checks: Option<i64>,
    pub successful_checks: Option<i64>,
    pub failed_checks: Option<i64>,
    pub average_response_time_ms: Option<f64>,
    pub active_alerts: Option<i64>,
    pub by_agent: Option<Vec<serde_json::Value>>,
}

/// Severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
}
/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: AlertStatus,
    pub severity: Severity,
    pub agent_id: Option<String>,
    pub target: Option<String>,
    pub metric_type: Option<String>,
    pub threshold_value: Option<f64>,
    pub current_value: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// AlertRule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub condition: AlertCondition,
    pub severity: Severity,
    pub notifications: Vec<NotificationChannel>,
    /// Minimum time between repeat notifications
    pub cooldown_seconds: Option<i64>,
}

/// Operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    #[serde(rename = "greater_than")]
    GreaterThan,
    #[serde(rename = "less_than")]
    LessThan,
    #[serde(rename = "equals")]
    Equals,
    #[serde(rename = "not_equals")]
    NotEquals,
}
/// Aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Aggregation {
    #[serde(rename = "avg")]
    Avg,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "count")]
    Count,
}
/// Filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filters {
    pub agent_ids: Option<Vec<String>>,
    pub targets: Option<Vec<String>>,
    pub metric_types: Option<Vec<String>>,
}

/// AlertCondition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric: Metric,
    pub operator: Operator,
    pub threshold: f64,
    /// Condition must be true for this duration before alerting
    pub duration_seconds: Option<i64>,
    pub aggregation: Option<Aggregation>,
    pub filters: Option<Filters>,
}

/// NotificationChannel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    #[serde(rename = "type")]
    pub r#type: Type,
    /// Channel-specific configuration
    pub configuration: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// TokenResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// Server-managed opaque session token. Use as a Bearer token in the
    /// Authorization header for all subsequent requests.
    /// Format: st_live_<hex> (production) or st_test_<hex> (dev/test).
    pub opaque_token: String,
    /// Hard expiry time (ISO 8601). The session will never be valid after
    /// this timestamp regardless of activity. Use this to schedule a
    /// proactive re-login in the client (e.g. at the midpoint).
    pub expires_at: DateTime<Utc>,
}

/// UserInfo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Subject identifier (user ID)
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub organization_id: Option<Uuid>,
    pub roles: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,
}

/// User
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub roles: Option<Vec<String>>,
    pub organization_id: Option<Uuid>,
    pub status: Option<UserStatus>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    /// Which OAuth2 provider the user authenticated with
    pub identity_provider: Option<String>,
    /// User ID from external identity provider
    pub external_id: Option<String>,
}

/// CreateUserRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
    /// Required for super admins creating users in other orgs
    pub organization_id: Option<Uuid>,
}

/// UpdateUserRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub status: Option<UpdateUserStatus>,
    pub roles: Option<Vec<String>>,
}

/// Plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Plan {
    #[serde(rename = "free")]
    Free,
    #[serde(rename = "professional")]
    Professional,
    #[serde(rename = "enterprise")]
    Enterprise,
}
/// Settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub max_agents: Option<i64>,
    pub retention_days: Option<i64>,
    pub features: Option<Vec<String>>,
}

/// Organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub status: Option<OrganizationStatus>,
    pub plan: Option<Plan>,
    pub settings: Option<Settings>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// CreateOrganizationRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: Option<String>,
    pub plan: Option<Plan>,
}

/// SystemStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub status: SystemHealthStatus,
    pub timestamp: DateTime<Utc>,
    pub version: Option<String>,
    pub uptime_seconds: Option<i64>,
    pub components: Option<ComponentsStatus>,
}

/// ComponentsStatus
pub type ComponentsStatus = std::collections::HashMap<String, ComponentStatus>;

/// ComponentStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub status: ComponentHealthStatus,
    pub message: Option<String>,
    pub response_time_ms: Option<f64>,
}

/// Health status of the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
}
/// Status of a monitoring check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricStatus {
    #[serde(rename = "reachable")]
    Reachable,
    #[serde(rename = "unreachable")]
    Unreachable,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unknown")]
    Unknown,
}
/// Status of agent registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationStatus {
    #[serde(rename = "pending_claim")]
    PendingClaim,
}
/// IP address family (IPv4 or IPv6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpAddressFamily {
    #[serde(rename = "ipv4")]
    Ipv4,
    #[serde(rename = "ipv6")]
    Ipv6,
}
/// Pending claim status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimStatusPendingEnum {
    #[serde(rename = "pending_claim")]
    PendingClaim,
}
/// Claimed status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimStatusClaimedEnum {
    #[serde(rename = "claimed")]
    Claimed,
}
/// Status in claim response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimResponseStatus {
    #[serde(rename = "claimed")]
    Claimed,
}
/// Report acknowledgment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportAckStatus {
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "queued")]
    Queued,
}
/// Metric status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregatedMetricStatus {
    #[serde(rename = "reachable")]
    Reachable,
    #[serde(rename = "unreachable")]
    Unreachable,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unknown")]
    Unknown,
}
/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "acknowledged")]
    Acknowledged,
    #[serde(rename = "resolved")]
    Resolved,
}
/// User account status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "suspended")]
    Suspended,
}
/// Updated user status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateUserStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "suspended")]
    Suspended,
}
/// Organization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrganizationStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "suspended")]
    Suspended,
    #[serde(rename = "trial")]
    Trial,
}
/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unhealthy")]
    Unhealthy,
}
/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unhealthy")]
    Unhealthy,
}
/// UUID version 7 as per RFC 4122
pub type UUIDv7 = Uuid;

/// Pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: i64,
    pub page_size: i64,
    pub total_items: i64,
    pub total_pages: i64,
    pub has_next: Option<bool>,
    pub has_previous: Option<bool>,
}

/// Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub error: String,
    pub message: String,
    pub details: Option<Vec<serde_json::Value>>,
    pub request_id: Option<Uuid>,
    /// Link to relevant documentation
    pub documentation_url: Option<String>,
}

/// AcknowledgeAlertRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledgeAlertRequestBody {
    /// Optional note about the acknowledgment
    pub note: Option<String>,
}

/// OAuth2 grant type (only authorization_code is supported here)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrantType {
    #[serde(rename = "authorization_code")]
    AuthorizationCode,
}
/// Oauth2TokenRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oauth2TokenRequestBody {
    /// OAuth2 grant type (only authorization_code is supported here)
    pub grant_type: GrantType,
    /// Authorization code returned by the IDP callback
    pub code: String,
    /// Must exactly match the redirect_uri used in the authorization request
    pub redirect_uri: String,
    /// PKCE code verifier corresponding to the code_challenge sent at /authorize
    pub code_verifier: String,
}

/// Oauth2RevokeRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oauth2RevokeRequestBody {
    /// The opaque session token to revoke (as returned by /auth/oauth2/token or /auth/refresh)
    pub opaque_token: String,
}

/// LogoutRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequestBody {
    /// Optional URI to redirect to after IDP logout completes.
    /// Forwarded to the IDP end-session endpoint as post_logout_redirect_uri.
    pub post_logout_redirect_uri: Option<String>,
}

/// UpdateUserRolesRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRolesRequestBody {
    pub roles: Vec<String>,
}

/// GetWebSocketTokenRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWebSocketTokenRequestBody {
    /// List of topics to subscribe to
    pub subscriptions: Vec<String>,
    /// Optional filters for subscriptions
    pub filters: Option<Filters>,
}

/// RegisterAgentSelfRequest
#[derive(Debug, Clone, Serialize)]
pub struct RegisterAgentSelfRequest {
    pub body: AgentSelfRegistration,
}
/// PostClaimAgentRequest
#[derive(Debug, Clone, Serialize)]
pub struct PostClaimAgentRequest {
    pub body: ClaimAgentRequest,
}
/// SendAgentHeartbeatRequest
#[derive(Debug, Clone, Serialize)]
pub struct SendAgentHeartbeatRequest {
    pub body: AgentHeartbeat,
}
/// SubmitAgentResultsRequest
#[derive(Debug, Clone, Serialize)]
pub struct SubmitAgentResultsRequest {
    pub body: BatchMonitoringResults,
}
/// CreateAlertRequest
#[derive(Debug, Clone, Serialize)]
pub struct CreateAlertRequest {
    pub body: AlertRule,
}
/// AcknowledgeAlertRequest
#[derive(Debug, Clone, Serialize)]
pub struct AcknowledgeAlertRequest {
    pub body: AcknowledgeAlertRequestBody,
}
/// Oauth2TokenRequest
#[derive(Debug, Clone, Serialize)]
pub struct Oauth2TokenRequest {
    pub body: Oauth2TokenRequestBody,
}
/// Oauth2RevokeRequest
#[derive(Debug, Clone, Serialize)]
pub struct Oauth2RevokeRequest {
    pub body: Oauth2RevokeRequestBody,
}
/// LogoutRequest
#[derive(Debug, Clone, Serialize)]
pub struct LogoutRequest {
    pub body: LogoutRequestBody,
}
/// PostCreateUserRequest
#[derive(Debug, Clone, Serialize)]
pub struct PostCreateUserRequest {
    pub body: CreateUserRequest,
}
/// PatchUserRequest
#[derive(Debug, Clone, Serialize)]
pub struct PatchUserRequest {
    pub body: UpdateUserRequest,
}
/// UpdateUserRolesRequest
#[derive(Debug, Clone, Serialize)]
pub struct UpdateUserRolesRequest {
    pub body: UpdateUserRolesRequestBody,
}
/// PostCreateOrganizationRequest
#[derive(Debug, Clone, Serialize)]
pub struct PostCreateOrganizationRequest {
    pub body: CreateOrganizationRequest,
}
/// GetWebSocketTokenRequest
#[derive(Debug, Clone, Serialize)]
pub struct GetWebSocketTokenRequest {
    pub body: GetWebSocketTokenRequestBody,
}
/// Agent registration request created
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterAgentSelfResponse201 {
    pub body: AgentRegistrationResponse,
}
/// Agent registration request updated (idempotent)
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterAgentSelfResponse200 {
    pub body: AgentRegistrationResponse,
}
/// Claim status retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetAgentClaimStatusResponse200 {
    pub body: serde_json::Value,
}
/// Agent registration not found or expired
#[derive(Debug, Clone, Deserialize)]
pub struct GetAgentClaimStatusResponse404 {
    pub body: Error,
}
/// Agent claimed successfully
#[derive(Debug, Clone, Deserialize)]
pub struct PostClaimAgentResponse200 {
    pub body: ClaimAgentResponse,
}
/// Forbidden - Invalid claim token or expired
#[derive(Debug, Clone, Deserialize)]
pub struct PostClaimAgentResponse403 {
    pub body: Error,
}
/// Agent registration not found
#[derive(Debug, Clone, Deserialize)]
pub struct PostClaimAgentResponse404 {
    pub body: Error,
}
/// Agent already claimed
#[derive(Debug, Clone, Deserialize)]
pub struct PostClaimAgentResponse409 {
    pub body: Error,
}
/// Configuration retrieved successfully
#[derive(Debug, Clone, Deserialize)]
pub struct GetAgentConfigurationResponse200 {
    pub body: AgentConfig,
}
/// Batch accepted for processing
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitAgentResultsResponse202 {
    pub body: ResultsBatchAcknowledgment,
}
/// Agent list retrieved successfully
#[derive(Debug, Clone, Deserialize)]
pub struct ListAgentsResponse200 {
    pub body: AgentListResponse,
}
/// Report generated successfully
#[derive(Debug, Clone, Deserialize)]
pub struct GetResultReportResponse200 {
    pub body: ResultReport,
}
/// Metrics retrieved successfully
#[derive(Debug, Clone, Deserialize)]
pub struct GetMetricsResponse200 {
    pub body: MetricsResponse,
}
/// Summary statistics retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetSummaryResponse200 {
    pub body: SummaryStatistics,
}
/// Alerts retrieved successfully
#[derive(Debug, Clone, Deserialize)]
pub struct ListAlertsResponse200 {
    pub body: serde_json::Value,
}
/// Alert rule created
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAlertResponse201 {
    pub body: AlertRule,
}
/// Alert details retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetAlertResponse200 {
    pub body: Alert,
}
/// Alert acknowledged
#[derive(Debug, Clone, Deserialize)]
pub struct AcknowledgeAlertResponse200 {
    pub body: Alert,
}
/// Session created; opaque token returned
#[derive(Debug, Clone, Deserialize)]
pub struct Oauth2TokenResponse200 {
    pub body: TokenResponse,
}
/// Session revoked successfully.
#[derive(Debug, Clone, Deserialize)]
pub struct Oauth2RevokeResponse200 {
    pub body: serde_json::Value,
}
/// New session token issued
#[derive(Debug, Clone, Deserialize)]
pub struct AuthRefreshResponse200 {
    pub body: TokenResponse,
}
/// User information retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserInfoResponse200 {
    pub body: UserInfo,
}
/// Logout acknowledged (providers without end-session endpoint, e.g. GitHub).
#[derive(Debug, Clone, Deserialize)]
pub struct LogoutResponse200 {
    pub body: serde_json::Value,
}
/// Users retrieved successfully
#[derive(Debug, Clone, Deserialize)]
pub struct ListUsersResponse200 {
    pub body: serde_json::Value,
}
/// User created successfully
#[derive(Debug, Clone, Deserialize)]
pub struct PostCreateUserResponse201 {
    pub body: User,
}
/// User details retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserResponse200 {
    pub body: User,
}
/// User updated successfully
#[derive(Debug, Clone, Deserialize)]
pub struct PatchUserResponse200 {
    pub body: User,
}
/// Roles updated successfully
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRolesResponse200 {
    pub body: User,
}
/// Organizations retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct ListOrganizationsResponse200 {
    pub body: serde_json::Value,
}
/// Organization created
#[derive(Debug, Clone, Deserialize)]
pub struct PostCreateOrganizationResponse201 {
    pub body: Organization,
}
/// Organization details retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetOrganizationResponse200 {
    pub body: Organization,
}
/// WebSocket token generated
#[derive(Debug, Clone, Deserialize)]
pub struct GetWebSocketTokenResponse200 {
    pub body: serde_json::Value,
}
/// Server is healthy
#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckResponse200 {
    pub body: SystemStatus,
}
/// Server is unhealthy
#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckResponse503 {
    pub body: SystemStatus,
}
/// Metrics in Prometheus format
#[derive(Debug, Clone, Deserialize)]
pub struct PrometheusMetricsResponse200 {
    pub body: String,
}
