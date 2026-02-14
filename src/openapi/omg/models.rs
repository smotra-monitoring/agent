use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type UUIDv7 = Uuid; // Placeholder for actual UUIDv7 implementation

/// AgentStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub is_running: bool,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub checks_performed: i64,
    pub checks_successful: i64,
    pub checks_failed: i64,
    pub last_report_at: DateTime<Utc>,
    pub failed_report_count: i64,
    pub server_connected: bool,
    pub cached_reports: i64,
}

/// AgentConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub version: i64,
    pub agent_id: UUIDv7,
    pub agent_name: String,
    pub tags: Option<Vec<String>>,
    pub monitoring: MonitoringConfig,
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub endpoints: Vec<Endpoint>,
}

/// MonitoringConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub interval_secs: i64,
    pub timeout_secs: i64,
    pub ping_count: i64,
    pub max_concurrent: i64,
    pub traceroute_on_failure: bool,
    pub traceroute_max_hops: i64,
}

/// ServerConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub report_interval_secs: i64,
    pub heartbeat_interval_secs: i64,
    pub verify_tls: bool,
    pub timeout_secs: i64,
    pub retry_attempts: i64,
}

/// StorageConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub cache_dir: String,
    pub max_cached_results: i64,
    pub max_cache_age_secs: i64,
}

/// Endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: UUIDv7,
    pub address: String,
    pub port: Option<i64>,
    pub enabled: bool,
    pub tags: Option<Vec<String>>,
}

/// MonitoringResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringResult {
    pub id: String,
    pub agent_id: String,
    pub target: Endpoint,
    pub check_type: CheckType,
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

/// PluginResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub plugin_name: Option<String>,
    pub plugin_version: Option<String>,
    pub success: Option<bool>,
    pub response_time_ms: Option<f64>,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}

/// Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
}
/// AgentHeartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    pub timestamp: DateTime<Utc>,
    pub status: Option<Status>,
    pub cpu_usage_percent: Option<f64>,
    pub memory_usage_mb: Option<f64>,
}

/// Metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub target: String,
    pub status: Status,
    pub response_time_ms: Option<f64>,
    pub packet_loss_percent: Option<f64>,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// AgentRegistration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub hostname: String,
    pub ip_address: String,
    pub agent_version: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub tags: Option<serde_json::Value>,
}

/// AgentCredentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCredentials {
    pub agent_id: UUIDv7,
    pub api_key: String,
    pub configuration_url: Option<String>,
}

/// AgentSelfRegistration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSelfRegistration {
    #[serde(rename = "agentId")]
    pub agent_id: UUIDv7,
    #[serde(rename = "claimTokenHash")]
    pub claim_token_hash: String,
    pub hostname: String,
    #[serde(rename = "agentVersion")]
    pub agent_version: String,
}

/// AgentRegistrationResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResponse {
    pub status: Status,
    #[serde(rename = "pollUrl")]
    pub poll_url: String,
    #[serde(rename = "claimUrl")]
    pub claim_url: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
}

/// ClaimStatusPending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimStatusPending {
    pub status: Status,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
}

/// ClaimStatusClaimed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimStatusClaimed {
    pub status: Status,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "configUrl")]
    pub config_url: String,
}

/// ClaimAgentRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAgentRequest {
    #[serde(rename = "agentId")]
    pub agent_id: UUIDv7,
    #[serde(rename = "claimToken")]
    pub claim_token: String,
    #[serde(rename = "sectionId")]
    pub section_id: UUIDv7,
    pub name: Option<String>,
}

/// ClaimAgentResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAgentResponse {
    #[serde(rename = "agentId")]
    pub agent_id: UUIDv7,
    pub status: Status,
    pub message: String,
}

/// PluginConfiguration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfiguration {
    pub name: String,
    pub version: Option<String>,
    pub enabled: bool,
    pub configuration: Option<serde_json::Value>,
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
    pub status: Status,
    pub received_at: DateTime<Utc>,
    pub configuration_version: Option<i64>,
    pub update_available: Option<bool>,
}

/// ResultReport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultReport {
    pub time_range: serde_json::Value,
    pub aggregation: Option<String>,
    pub data: Vec<AggregatedMetric>,
    pub pagination: Pagination,
    pub metadata: Option<serde_json::Value>,
}

/// AggregatedMetric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub timestamp: DateTime<Utc>,
    pub agent_id: UUIDv7,
    pub hostname: Option<String>,
    pub target: String,
    pub metric_type: String,
    pub status: Option<Status>,
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
    pub status: Status,
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
    pub cooldown_seconds: Option<i64>,
}

/// Operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    #[serde(rename = "greater_than")]
    Greater_than,
    #[serde(rename = "less_than")]
    Less_than,
    #[serde(rename = "equals")]
    Equals,
    #[serde(rename = "not_equals")]
    Not_equals,
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
/// AlertCondition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric: Metric,
    pub operator: Operator,
    pub threshold: f64,
    pub duration_seconds: Option<i64>,
    pub aggregation: Option<Aggregation>,
    pub filters: Option<serde_json::Value>,
}

/// NotificationChannel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    #[serde(rename = "type")]
    pub r#type: Type,
    pub configuration: Option<serde_json::Value>,
}

/// GrantType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrantType {
    #[serde(rename = "authorization_code")]
    Authorization_code,
}
/// AuthorizationCodeTokenRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCodeTokenRequest {
    pub grant_type: GrantType,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
}

/// RefreshTokenRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub grant_type: GrantType,
    pub refresh_token: String,
    pub scope: Option<String>,
}

/// ClientCredentialsTokenRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCredentialsTokenRequest {
    pub grant_type: GrantType,
    pub client_id: String,
    pub client_secret: String,
    pub scope: Option<String>,
}

/// TokenResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

/// UserInfo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
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
    pub status: Option<Status>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub identity_provider: Option<String>,
    pub external_id: Option<String>,
}

/// CreateUserRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
    pub organization_id: Option<Uuid>,
}

/// UpdateUserRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub status: Option<Status>,
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
/// Organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub status: Option<Status>,
    pub plan: Option<Plan>,
    pub settings: Option<serde_json::Value>,
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

/// HealthStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: Status,
    pub timestamp: DateTime<Utc>,
    pub version: Option<String>,
    pub uptime_seconds: Option<i64>,
    pub components: Option<serde_json::Value>,
}

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
    pub documentation_url: Option<String>,
}

/// AcknowledgeAlertRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledgeAlertRequestBody {
    pub note: Option<String>,
}

/// Hint about token type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenTypeHint {
    #[serde(rename = "access_token")]
    Access_token,
    #[serde(rename = "refresh_token")]
    Refresh_token,
}
/// Oauth2RevokeRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oauth2RevokeRequestBody {
    pub token: String,
    pub token_type_hint: Option<TokenTypeHint>,
}

/// LogoutRequestBody
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequestBody {
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
    pub subscriptions: Vec<String>,
    pub filters: Option<serde_json::Value>,
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
pub struct ClaimAgentResponse200 {
    pub body: ClaimAgentResponse,
}
/// Forbidden - Invalid claim token or expired
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimAgentResponse403 {
    pub body: Error,
}
/// Agent registration not found
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimAgentResponse404 {
    pub body: Error,
}
/// Agent already claimed
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimAgentResponse409 {
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
pub struct CreateUserResponse201 {
    pub body: User,
}
/// User details retrieved
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserResponse200 {
    pub body: User,
}
/// User updated successfully
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserResponse200 {
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
pub struct CreateOrganizationResponse201 {
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
    pub body: HealthStatus,
}
/// Server is unhealthy
#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckResponse503 {
    pub body: HealthStatus,
}
/// Metrics in Prometheus format
#[derive(Debug, Clone, Deserialize)]
pub struct PrometheusMetricsResponse200 {
    pub body: String,
}
