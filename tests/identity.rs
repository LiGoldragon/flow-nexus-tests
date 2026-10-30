use flow_nexus_runtime::state::{Mutatable, RecordState};
use flow_nexus_types::evidence::{
    AttemptedAction, CorrelationLocator, CreationIntent, CreationLocator, EndPhase, Invariant,
    LaunchPhase, Mutation, MutationBatch, ProcessIncarnation, PromptContent, Record, RequestId,
    ResourceEvidence, ResolvedLaunch, ServerBinding,
};
use flow_nexus_types::memory::{AddressReservation, EndAttempt, Flow, LaunchAttempt, Permaflow, RequestAllocator};
use flow_nexus_types::{
    Address, Aspect, Declaration, ExecutablePath, HarnessProfile, HarnessProfileName,
    HarnessSettings, HarnessVersion, IdentityAlphabet, IdentityPolicy, Layer, ModelIdentifier,
    ModelProfile, ModelProfileName, NamespaceIdentity, State, Topic, Tokens,
};

struct IdentityFixture {
    address: Address,
    locator: CreationLocator,
}

trait FixtureFactory {
    fn topic(topic: &str) -> Self;
    fn resolved(&self, address: Address) -> ResolvedLaunch;
    fn launch(&self, request_id: i64, flow_id: &str, address: Address) -> LaunchAttempt;
    fn reservation(&self, request_id: i64, flow_id: &str, address: Address) -> AddressReservation;
}

impl Default for IdentityFixture {
    fn default() -> Self {
        Self::topic("identity")
    }
}

impl FixtureFactory for IdentityFixture {
    fn topic(topic: &str) -> Self {
        Self {
            address: Address { aspect: Aspect::Mind, topic: Topic(topic.to_owned()), layer: Layer::Tertiary },
            locator: CreationLocator {
                server_binding: ServerBinding {
                    herdr_socket_path: flow_nexus_types::HerdrSocketPath("socket".to_owned()),
                    herdr_session_name: flow_nexus_types::HerdrSessionName("session".to_owned()),
                    process_incarnation: ProcessIncarnation {
                        host_boot_identity: flow_nexus_types::evidence::HostBootIdentity("boot".to_owned()),
                        pid_namespace_identity: flow_nexus_types::evidence::PidNamespaceIdentity("pidns".to_owned()),
                        process_id: flow_nexus_types::evidence::ProcessId(1),
                        process_start_ticks: flow_nexus_types::evidence::ProcessStartTicks(1),
                    },
                },
                correlation_locator: CorrelationLocator("correlation".to_owned()),
            },
        }
    }

    fn resolved(&self, address: Address) -> ResolvedLaunch {
        let harness_name = HarnessProfileName("harness".to_owned());
        ResolvedLaunch {
            declaration: Declaration { address, module_id_vector: vec![], harness_module_name_vector: vec![] },
            model_profile: ModelProfile {
                model_profile_name: ModelProfileName("model".to_owned()),
                model_identifier: ModelIdentifier("model-id".to_owned()),
                harness_profile_name: harness_name.clone(),
                tokens: Tokens(1),
            },
            harness_profile: HarnessProfile {
                harness_profile_name: harness_name,
                harness_version: HarnessVersion("1".to_owned()),
                executable_path: ExecutablePath("harness".to_owned()),
                harness_module_vector: vec![],
                harness_settings: HarnessSettings::CodexUnavailable,
            },
            identity_policy: IdentityPolicy { identity_alphabet: IdentityAlphabet("abc".to_owned()) },
            capsule_configuration: flow_nexus_types::CapsuleConfiguration {
                home_path: flow_nexus_types::HomePath("/tmp".to_owned()),
                transcript_directory: flow_nexus_types::TranscriptDirectory("/tmp".to_owned()),
                working_directory: flow_nexus_types::WorkingDirectory("/tmp".to_owned()),
                herdr_executable_path: flow_nexus_types::HerdrExecutablePath("herdr".to_owned()),
                herdr_socket_path: flow_nexus_types::HerdrSocketPath("socket".to_owned()),
                herdr_session_name: flow_nexus_types::HerdrSessionName("session".to_owned()),
                namespace_identity: NamespaceIdentity("namespace".to_owned()),
            },
            module_content_vector: vec![],
            prompt_content: PromptContent {
                system_prompt: flow_nexus_types::evidence::SystemPrompt("system".to_owned()),
                first_prompt: flow_nexus_types::evidence::FirstPrompt("first".to_owned()),
            },
        }
    }

    fn launch(&self, request_id: i64, flow_id: &str, address: Address) -> LaunchAttempt {
        LaunchAttempt {
            request_id: RequestId(request_id),
            flow_id: flow_nexus_types::FlowId(flow_id.to_owned()),
            address: address.clone(),
            resolved_launch: self.resolved(address),
            resource_evidence: ResourceEvidence::NoneConfirmed(self.locator.clone()),
            creation_intent: CreationIntent { creation_locator: self.locator.clone(), attempted_action: AttemptedAction::CreatePane },
            launch_phase: LaunchPhase::Reserved,
        }
    }

    fn reservation(&self, request_id: i64, flow_id: &str, address: Address) -> AddressReservation {
        AddressReservation { address, flow_id: flow_nexus_types::FlowId(flow_id.to_owned()), request_id: RequestId(request_id) }
    }
}

#[test]
fn one_flow_id_cannot_be_reserved_at_two_addresses() {
    let first = IdentityFixture::topic("one");
    let second = IdentityFixture::topic("two");
    let batch = MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::LaunchAttempt(first.launch(1, "sameaa", first.address.clone()))),
        Mutation::Put(Record::AddressReservation(first.reservation(1, "sameaa", first.address.clone()))),
        Mutation::Put(Record::LaunchAttempt(second.launch(2, "sameaa", second.address.clone()))),
        Mutation::Put(Record::AddressReservation(second.reservation(2, "sameaa", second.address.clone()))),
    ]);
    assert_eq!(RecordState::default().mutate(&batch), Err(Invariant::DuplicateIdentity(flow_nexus_types::FlowId("sameaa".to_owned()))));
}

#[test]
fn launch_and_end_attempts_cannot_share_request_id() {
    let fixture = IdentityFixture::default();
    let flow_id = flow_nexus_types::FlowId("flowaa".to_owned());
    let batch = MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::Flow(Flow { flow_id: flow_id.clone(), address: fixture.address.clone(), session_id_option: None, state: State::Ended, flow_id_option: None, transcript_path_option: None })),
        Mutation::Put(Record::Permaflow(Permaflow { address: fixture.address.clone(), flow_id_vector: vec![flow_id] })),
        Mutation::Put(Record::LaunchAttempt(fixture.launch(7, "flowaa", fixture.address.clone()))),
        Mutation::Put(Record::AddressReservation(fixture.reservation(7, "flowaa", fixture.address.clone()))),
        Mutation::Put(Record::EndAttempt(EndAttempt { request_id: RequestId(7), flow_id: flow_nexus_types::FlowId("flowaa".to_owned()), end_phase: EndPhase::Complete })),
    ]);
    assert_eq!(RecordState::default().mutate(&batch), Err(Invariant::DuplicateRequest(RequestId(7))));
}

#[test]
fn completed_launch_requires_its_flow_row() {
    let fixture = IdentityFixture::default();
    let mut attempt = fixture.launch(9, "missaa", fixture.address.clone());
    attempt.launch_phase = LaunchPhase::Complete;
    let batch = MutationBatch(vec![Mutation::Put(Record::LaunchAttempt(attempt))]);
    assert_eq!(RecordState::default().mutate(&batch), Err(Invariant::DanglingReference(flow_nexus_types::evidence::RecordKey::Flow(flow_nexus_types::FlowId("missaa".to_owned())))));
}
