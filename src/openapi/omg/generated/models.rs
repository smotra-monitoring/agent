//!
//! Generated from an OAS specification by openapi-model-generator(v0.5.1)
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// AgentStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Unique identifier for the agent
    pub agent_id: String,
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
    /// Number of reports cached locally on the agent
    pub cached_reports: i64,
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

/// Endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: UUIDv7,
    /// IP address, hostname, or URL
    pub address: String,
    pub port: Option<i64>,
    pub enabled: bool,
    /// Tags associated with the target
    pub tags: Option<Vec<String>>,
}

/// MonitoringResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringResult {
    /// Unique identifier for the monitoring result
    pub id: String,
    /// Unique identifier for the agent
    pub agent_id: String,
    pub target: Endpoint,
    pub check_type: CheckType,
    /// Timestamp when the report was generated (RFC3339)
    pub timestamp: DateTime<Utc>,
}

/// Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "ping")]
    Ping,
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
/// PingCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingCheck {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub result: PingResult,
}

/// PingResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub resolved_ip: Option<String>,
    pub successes: Option<i64>,
    pub failures: Option<i64>,
    pub success_latencies: Option<Vec<f64>>,
    pub avg_response_time_ms: Option<f64>,
    pub errors: Option<Vec<String>>,
}

/// TracerouteCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteCheck {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub result: TracerouteResult,
}

/// TracerouteResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteResult {
    pub hops: Option<Vec<TracerouteHop>>,
    pub target_reached: Option<bool>,
    pub total_time_ms: Option<f64>,
    pub errors: Option<Vec<String>>,
}

/// TracerouteHop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteHop {
    pub hop: Option<i64>,
    pub address: Option<String>,
    pub response_time_ms: Option<f64>,
    pub hostname: Option<String>,
}

/// TcpConnectCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnectCheck {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub result: TcpConnectResult,
}

/// TcpConnectResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnectResult {
    pub connected: Option<bool>,
    pub connect_time_ms: Option<f64>,
    pub error: Option<String>,
    pub resolved_ip: Option<String>,
}

/// UdpConnectCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConnectCheck {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub result: UdpConnectResult,
}

/// UdpConnectResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConnectResult {
    pub probe_successful: Option<bool>,
    pub response_time_ms: Option<f64>,
    pub error: Option<String>,
    pub resolved_ip: Option<String>,
}

/// HttpGetCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGetCheck {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub result: HttpGetResult,
}

/// HttpGetResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpGetResult {
    pub status_code: Option<i64>,
    pub response_time_ms: Option<f64>,
    pub response_size_bytes: Option<i64>,
    pub error: Option<String>,
    pub success: Option<bool>,
}

/// PluginCheck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCheck {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub result: PluginResult,
}

/// Data
pub type Data = std::collections::HashMap<String, String>;

/// PluginResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub plugin_name: Option<String>,
    pub plugin_version: Option<String>,
    pub success: Option<bool>,
    pub response_time_ms: Option<f64>,
    pub error: Option<String>,
    pub data: Option<std::collections::HashMap<String, String>>,
}

/// AgentHeartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    pub timestamp: DateTime<Utc>,
    pub status: Option<AgentHealthStatus>,
    pub cpu_usage_percent: Option<f64>,
    pub memory_usage_mb: Option<f64>,
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

/// ReportAcknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportAcknowledgment {
    pub request_id: UUIDv7,
    pub status: ReportAckStatus,
    pub received_at: DateTime<Utc>,
    /// Latest configuration version available
    pub configuration_version: Option<i64>,
    /// Whether agent update is available
    pub update_available: Option<bool>,
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

/// GrantType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrantType {
    #[serde(rename = "authorization_code")]
    AuthorizationCode,
}
/// AuthorizationCodeTokenRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCodeTokenRequest {
    pub grant_type: GrantType,
    /// Authorization code from callback
    pub code: String,
    /// Must match original authorization request
    pub redirect_uri: String,
    pub client_id: String,
    /// Required for confidential clients
    pub client_secret: Option<String>,
    /// PKCE code verifier
    pub code_verifier: Option<String>,
}

/// RefreshTokenRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub grant_type: GrantType,
    pub refresh_token: String,
    /// Optional scope restriction
    pub scope: Option<String>,
}

/// ClientCredentialsTokenRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCredentialsTokenRequest {
    pub grant_type: GrantType,
    pub client_id: String,
    pub client_secret: String,
    /// Space-separated list of requested scopes
    pub scope: Option<String>,
}

/// TokenResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// JWT access token
    pub access_token: String,
    pub token_type: String,
    /// Token lifetime in seconds
    pub expires_in: i64,
    /// Refresh token (only for authorization_code grant)
    pub refresh_token: Option<String>,
    /// Space-separated list of granted scopes
    pub scope: Option<String>,
    /// OpenID Connect ID token (if openid scope requested)
    pub id_token: Option<String>,
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

/// Hint about token type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenTypeHint {
    #[serde(rename = "access_token")]
    AccessToken,
    #[serde(rename = "refresh_token")]
    RefreshToken,
}
/// Oauth2RevokeRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oauth2RevokeRequestBody {
    /// Token to revoke
    pub token: String,
    /// Hint about token type
    pub token_type_hint: Option<TokenTypeHint>,
}

/// LogoutRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequestBody {
    /// Where to redirect after logout
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

/// SubmitAgentStatusRequest
#[derive(Debug, Clone, Serialize)]
pub struct SubmitAgentStatusRequest {
    pub body: AgentStatus,
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
/// UpdateAgentConfigurationRequest
#[derive(Debug, Clone, Serialize)]
pub struct UpdateAgentConfigurationRequest {
    pub body: AgentConfig,
}
/// SendAgentHeartbeatRequest
#[derive(Debug, Clone, Serialize)]
pub struct SendAgentHeartbeatRequest {
    pub body: AgentHeartbeat,
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
    pub body: serde_json::Value,
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
/// Report accepted for processing
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitAgentStatusResponse202 {
    pub body: ReportAcknowledgment,
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
/// Configuration updated successfully
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAgentConfigurationResponse200 {
    pub body: AgentConfig,
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
/// Tokens issued successfully
#[derive(Debug, Clone, Deserialize)]
pub struct Oauth2TokenResponse200 {
    pub body: TokenResponse,
}
/// User information retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserInfoResponse200 {
    pub body: UserInfo,
}
/// Logout successful
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
