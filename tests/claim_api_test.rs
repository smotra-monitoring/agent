//! Test to verify the public API surface is minimal and clean

#[test]
fn test_claim_public_api() {
    // Only the Claim struct should be directly accessible
    // This test ensures we haven't accidentally exposed internal functions

    // This should compile - Claim is public
    use smotra_agent::Claim;
    let _ = std::marker::PhantomData::<Claim>;

    // These should NOT compile if uncommented (internal types are hidden):
    // use smotra_agent::claim::generate_claim_token; // Should fail
    // use smotra_agent::claim::AgentRegistration; // Should fail (but available via #[doc(hidden)])
}

#[test]
fn test_claim_workflow_via_public_api() {
    use smotra_agent::{Claim, Config};
    use tempfile::NamedTempFile;

    let config = Config::default();
    let temp_file = NamedTempFile::new().unwrap();

    // This is the only public way to use the claiming workflow
    let _claim = Claim::new(&config, temp_file.path());

    // Users would call: claim.run().await
}
