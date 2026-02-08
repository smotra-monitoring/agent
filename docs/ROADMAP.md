### Done
- [X] Convert Config.agent_id from String to Uuid
- [X] Look at run_claiming_workflow to remove redundant setting of config.agent_id
- [X] Would it be more idiomatic to set default for Confg.agent_id to Uuid::now_v7() instead of Uuid::nil() ? This will avoid unnecessary check for nil UUID in the rest of the codebase. what are the pros and cons ? 
- [X] Convert MonitorigResult.agent_id from String to Uuid
- [X] Sanity check for agent_id in PingChecker.check() and HeartbeatSender.send_heartbeat().
- [X] Convert AgentStatus.agent_id from String to Uuid
- [X] Refactor run_claiming_workflow. Create Claim object to encapsulate claiming logic.
- [X] in lib.rs try to make following modules private by default and only expose necessary types/functions at the top level:
  - agent_config
  - monitor
  - plugin
  - reporter
- [X] claim::AgentRegistration.agent_version should come from Cargo.toml at build time using env! macro. (double check other places where version string is used)



### In this PR

- [ ] Add copilot-instructions to follow Rust patterns (Builder, StateType, Factory, etc.) where applicable.
- [ ] Add copilot-instructions to add module structure mod.rs files should contain only module re-exports.
- [ ] Add copilot-instructions to make method public only if necessary, default must be private. Do not proliferate pub fn.
- [ ] Add copilot-instructions to document all features in folder docs/ with examples.

- [ ] Move /api/v1/ to api.smotra.net/v1/

- [ ] Generate README.md for git repository with usage examples.

- [ ] Replace authorization from Bearer to X-API-KEY header (it is incorrect in heartbeat.rs send_heartbeat, check other places as well).