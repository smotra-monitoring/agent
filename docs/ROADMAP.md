# Smotra Agent - Development Roadmap

This document outlines the development roadmap for the Smotra monitoring agent, including completed milestones, short-term priorities, and long-term vision.

---

## Current PR
- [ ] Move load_and_validate_config to AgentConfig.loader
- [ ] Refactor running part of first conifg load into load_and_validate_config. Replace flow if api_key is missing to check of returned error type Error::MissingApiKey.
      Current flow:
      - load config
      - if api_key is missing, start claiming workflow
      - validate config
      New flow:
      - load and validate config in one step
      - if api_key is missing, return Error::MissingApiKey, start claiming workflow in main.rs, then call load_and_validate_config again to load the config with api_key and validate it

- [ ] Refactor main gen_config. Move it to a dedicated function.
- [ ] Rename src/monitor/coordinator.rs -> src/monitor/server.rs, due to other similar files like src/monitor/server.rs and src/reporter/server.rs

- [ ] Refactor async loops in main.rs to follow the same principle as other async loops in the codebase (check agent.rs).

- [ ] IMO Agent::reload_config make an assumption that all async loops will pick-up config change because they programmed with reference to self.config. This is not true. Config cloned by value. DOUBLE CHECK !!! Potentially refactor rest of the green threads to use config reference instead of cloning it.

## ✅ Completed Milestones

### Core Infrastructure (v0.1.0)
- [X] Core agent library with lifecycle management
- [X] ICMP ping monitoring with surge-ping integration
- [X] Configuration system with TOML support and version tracking
- [X] Secure configuration persistence with file permissions (0600 on Unix)
- [X] Agent claiming workflow with zero-config onboarding
  - [X] Cryptographically secure token generation (SHA-256)
  - [X] Self-registration with retry logic
  - [X] Claim status polling
  - [X] API key delivery and persistence
- [X] Heartbeat reporting with system metrics (CPU, memory, uptime)
- [X] Server communication with X-API-KEY authentication
- [X] Plugin system foundation (trait and registry)
- [X] Interactive TUI with Ratatui (agent-cli)
- [X] Concurrent monitoring with semaphore limits
- [X] Result type definitions for all planned check types

### Code Quality & Architecture
- [X] Convert agent_id from String to Uuid throughout codebase
- [X] Refactor claiming workflow into dedicated Claim orchestrator
- [X] Module visibility cleanup (private by default)
- [X] Agent version from Cargo.toml at build time
- [X] Replace Bearer token with X-API-KEY header authentication
- [X] Comprehensive documentation in docs/ folder
- [X] Example code with proper naming convention (XXX_name.rs)
- [X] Copilot instructions for Rust patterns and best practices

### Testing
- [X] Unit tests for token generation and hashing
- [X] Unit tests for configuration types
- [X] Integration tests for claiming workflow with mock server
- [X] Integration tests for heartbeat functionality

---

## 🎯 Short-Term Goals (Next 1-3 Months)

### Priority 1: Complete Core Check Types
**Goal**: Implement all planned monitoring check types

- [ ] **Traceroute Implementation** (Week 1-2)
  - [ ] Implement TracerouteChecker using traceroute-rs or similar
  - [ ] Add concurrent traceroute execution
  - [ ] Unit tests for traceroute functionality
  - [ ] Integration tests with real network paths
  - [ ] Documentation and examples

- [ ] **TCP Connect Checker** (Week 2-3)
  - [ ] Implement TcpConnectChecker with tokio::net
  - [ ] Connection timeout handling
  - [ ] DNS resolution support
  - [ ] Unit and integration tests
  - [ ] Documentation

- [ ] **UDP Connect Checker** (Week 3-4)
  - [ ] Implement UdpConnectChecker
  - [ ] UDP probe/response mechanism
  - [ ] Timeout and retry logic
  - [ ] Tests and documentation

- [ ] **HTTP GET Checker** (Week 4-5)
  - [ ] Implement HttpGetChecker using reqwest
  - [ ] Status code validation
  - [ ] Response time and size metrics
  - [ ] TLS/SSL support
  - [ ] Tests and documentation

### Priority 2: Cache Persistence
**Goal**: Complete offline-first functionality

- [ ] **Implement Disk-Based Cache** (Week 5-6)
  - [ ] Design cache file format (consider SQLite or custom binary format)
  - [ ] Implement CacheManager with disk operations
  - [ ] Background cache flush mechanism
  - [ ] Cache size limits and rotation
  - [ ] Recovery from corrupted cache
  - [ ] Performance benchmarks
  - [ ] Tests for cache persistence and recovery

### Priority 3: API Documentation
**Goal**: Complete server API specification

- [✓] **OpenAPI Specification** (Week 6-7)
  - [✓] Document agent registration endpoints
  - [✓] Document claiming workflow endpoints
  - [✓] Document monitoring result submission
  - [✓] Document heartbeat reporting
  - [ ] Document configuration polling (future feature)
  - [✓] Add request/response examples
  - [✓] Set up OMG type generation from OpenAPI spec
  - [✓] Document type generation workflow in OPENAPI_CODE_GENERATION.md

### Priority 4: Enhanced Testing
**Goal**: Achieve 85%+ test coverage

- [ ] **Expand Test Coverage** (Ongoing)
  - [ ] Unit tests for all checker implementations
  - [ ] Integration tests for complete monitoring workflows
  - [ ] End-to-end tests with mock server
  - [ ] Performance and load tests
  - [ ] Network failure simulation tests
  - [ ] Configuration validation tests

---

## 🚀 Medium-Term Goals (3-6 Months)

### Enhanced Monitoring Capabilities
- [ ] **Dynamic Plugin Loading**
  - [ ] Design plugin ABI/API
  - [ ] Implement plugin discovery and loading from .so/.dll files
  - [ ] Plugin lifecycle management (load, unload, reload)
  - [ ] Plugin sandboxing and security
  - [ ] Plugin marketplace/registry concept
  - [ ] Example plugins (DNS, TLS cert validation, custom HTTP)

- [ ] **Advanced Metrics Collection**
  - [ ] Jitter calculation for ping checks
  - [ ] Packet loss patterns
  - [ ] Network path quality scoring
  - [ ] Historical trend analysis
  - [ ] Anomaly detection (basic)

- [ ] **Configuration Management**
  - [ ] Configuration hot-reload without restart
  - [ ] Configuration versioning and polling from server
  - [ ] Configuration validation with detailed error messages
  - [ ] Configuration migration tools
  - [ ] Per-endpoint configuration overrides

### Operational Excellence
- [ ] **Agent Auto-Updater**
  - [ ] Update check mechanism (poll server for new versions)
  - [ ] Binary download and verification (checksums, signatures)
  - [ ] Safe update process with rollback capability
  - [ ] Update scheduling (maintenance windows)
  - [ ] Update notification and approval workflow

- [ ] **Enhanced TUI**
  - [ ] Real-time graphs for latency trends
  - [ ] Historical data visualization
  - [ ] Log filtering and search
  - [ ] Configuration editor within TUI
  - [ ] Alert history view
  - [ ] Performance profiling view

- [ ] **Logging & Observability**
  - [ ] Structured logging with contextual fields
  - [ ] Log level adjustment at runtime
  - [ ] Log rotation and management
  - [ ] Export logs to common formats (JSON Lines, syslog)
  - [ ] Internal metrics endpoint (Prometheus format)
  - [ ] Health check HTTP endpoint

### Deployment & Distribution
- [ ] **Packaging**
  - [ ] Debian/Ubuntu packages (.deb)
  - [ ] RedHat/CentOS packages (.rpm)
  - [ ] Windows installer (MSI)
  - [ ] macOS installer (pkg)
  - [ ] Docker images with multi-architecture support
  - [ ] Kubernetes Helm charts
  - [ ] systemd service files
  - [ ] Windows service support

- [ ] **Installation Automation**
  - [X] Quick install script (curl | sh) - See [INSTALL_SCRIPT.md](INSTALL_SCRIPT.md) for implementation details
  - [ ] Ansible playbooks
  - [ ] Terraform modules
  - [ ] Configuration management integration (Puppet, Chef)

---

## 🌟 Long-Term Vision (6+ Months)

### Distributed Intelligence
- [ ] **Mesh Networking**
  - [ ] Agent-to-agent communication for distributed checks
  - [ ] Peer discovery and mesh formation based on tags
  - [ ] Distributed consensus for network state
  - [ ] Load balancing check responsibilities across mesh
  - [ ] Partition tolerance and self-healing

- [ ] **Edge Intelligence**
  - [ ] Local alerting without server (autonomous mode)
  - [ ] Intelligent check scheduling based on history
  - [ ] Predictive failure detection
  - [ ] Adaptive check intervals based on stability
  - [ ] Machine learning for anomaly detection (explore ML models)

### Advanced Features
- [ ] **Multi-Protocol Support**
  - [ ] gRPC for high-performance communication
  - [ ] MQTT for IoT deployments
  - [ ] WebSocket for real-time updates
  - [ ] Protocol auto-negotiation

- [ ] **Security Enhancements**
  - [ ] mTLS for agent-server communication
  - [ ] Certificate rotation
  - [ ] Role-based access control integration
  - [ ] Audit logging
  - [ ] Secrets management integration (Vault, etc.)

- [ ] **Extensibility**
  - [ ] Scripting support (Lua, Rhai) for custom checks
  - [ ] WebAssembly plugin support for sandboxed execution
  - [ ] Event-driven architecture with webhook support
  - [ ] Custom result processors and transformers

### Platform & Ecosystem
- [ ] **Multi-Tenant Support**
  - [ ] Organization/tenant isolation
  - [ ] Per-tenant configuration
  - [ ] Resource quotas and limits
  - [ ] Billing and usage tracking integration

- [ ] **Integration Ecosystem**
  - [ ] Grafana data source plugin
  - [ ] Prometheus exporter
  - [ ] Datadog integration
  - [ ] PagerDuty/OpsGenie alerting
  - [ ] Slack/Teams notifications
  - [ ] Webhook delivery for custom integrations

- [ ] **Developer Experience**
  - [ ] SDK for building custom agents
  - [ ] Mock server for development
  - [ ] Agent simulator for testing at scale
  - [ ] Performance profiling tools
  - [ ] Debug mode with enhanced logging

### Performance & Scalability
- [ ] **Optimization**
  - [ ] Memory usage optimization (streaming large results)
  - [ ] CPU usage benchmarking and optimization
  - [ ] Network bandwidth optimization (compression, batching)
  - [ ] Support for 10,000+ concurrent checks per agent
  - [ ] Sub-millisecond check initiation latency

- [ ] **High Availability**
  - [ ] Multiple server failover
  - [ ] Quorum-based health checks
  - [ ] Graceful degradation under load
  - [ ] Circuit breaker patterns

---

## 📋 Technical Debt & Maintenance

### Ongoing
- [ ] Maintain dependencies (security updates)
- [ ] Rust edition migrations
- [ ] Performance regression testing
- [ ] Documentation updates
- [ ] Code review and refactoring
- [ ] Community contributions review

### Future Considerations
- [ ] Evaluate alternative ping libraries for better performance
- [ ] Consider eBPF for advanced network monitoring (Linux)
- [ ] Explore QUIC protocol for faster server communication
- [ ] Investigate zero-copy techniques for large data transfers
- [ ] Research differential encoding for result compression

---

## 🤝 Community & Contributions

### Documentation
- [ ] Contribution guidelines
- [ ] Architecture deep-dive documentation
- [ ] Plugin development comprehensive guide
- [ ] Deployment best practices guide
- [ ] Troubleshooting guide
- [ ] FAQ section

### Community Building
- [ ] Public roadmap with community voting
- [ ] Monthly release notes and changelogs
- [ ] Example deployments and use cases
- [ ] Community plugins showcase
- [ ] Regular office hours or community calls

---

## 📊 Success Metrics

### Short-Term (3 months)
- All core check types implemented and tested
- 85%+ code coverage
- Complete OpenAPI specification
- Cache persistence functional
- < 0.1% crash rate in production

### Medium-Term (6 months)
- Plugin system with 5+ community plugins
- Auto-updater in production use
- Packages available for major platforms
- 1000+ active agents in production
- Sub-second agent startup time

### Long-Term (12+ months)
- Mesh networking with 100+ agent meshes
- 10,000+ active production agents
- 50+ community contributors
- 5+ major platform integrations
- Recognized as a leading open-source monitoring solution

---

*Last Updated: February 8, 2026*
*See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for current implementation details.* 