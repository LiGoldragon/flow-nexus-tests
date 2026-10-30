use flow_nexus_runtime::state::{Mutatable, RecordState};
use flow_nexus_types::evidence::{
    AttemptedAction, AdmittedRequest, ArrivalOrdinal, CorrelationLocator, CreationIntent, CreationLocator, EndPhase, Invariant,
    LaunchPhase, Mutation, MutationBatch, ProcessIncarnation, PromptContent, Record, RequestId,
    ResourceEvidence, ResolvedLaunch, ServerBinding, OriginAdmissionId,
};
use flow_nexus_types::memory::{AddressReservation, ArrivalAllocator, EndAttempt, Flow, LaunchAttempt, Permaflow, Queue, RequestAllocator};
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
            origin_admission_id_option: None,
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
fn matching_flow_history_reservation_and_launch_are_accepted() {
    let fixture = IdentityFixture::default();
    let flow_id = flow_nexus_types::FlowId("flowaa".to_owned());
    let batch = MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::Flow(Flow { flow_id: flow_id.clone(), address: fixture.address.clone(), session_id_option: None, state: State::Started, flow_id_option: None, transcript_path_option: None })),
        Mutation::Put(Record::Permaflow(Permaflow { address: fixture.address.clone(), flow_id_vector: vec![flow_id.clone()] })),
        Mutation::Put(Record::AddressReservation(fixture.reservation(7, "flowaa", fixture.address.clone()))),
        Mutation::Put(Record::LaunchAttempt(fixture.launch(7, "flowaa", fixture.address.clone()))),
    ]);
    assert!(RecordState::default().mutate(&batch).is_ok());
}

#[test]
fn existing_identity_collision_rejects_second_batch_without_change() {
    let fixture = IdentityFixture::default();
    let first = MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::AddressReservation(fixture.reservation(1, "flowaa", fixture.address.clone()))),
        Mutation::Put(Record::LaunchAttempt(fixture.launch(1, "flowaa", fixture.address.clone()))),
    ]);
    let state = RecordState::default().mutate(&first).unwrap();
    let before = state.clone();
    let other = IdentityFixture::topic("other");
    let second = MutationBatch(vec![
        Mutation::Put(Record::AddressReservation(other.reservation(2, "flowaa", other.address.clone()))),
        Mutation::Put(Record::LaunchAttempt(other.launch(2, "flowaa", other.address.clone()))),
    ]);
    assert_eq!(state.mutate(&second), Err(Invariant::DuplicateIdentity(flow_nexus_types::FlowId("flowaa".to_owned()))));
    assert_eq!(state, before);
}

#[test]
fn launch_request_id_reuse_is_rejected() {
    let fixture = IdentityFixture::default();
    let batch = MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::LaunchAttempt(fixture.launch(7, "flowaa", fixture.address.clone()))),
        Mutation::Put(Record::LaunchAttempt(fixture.launch(7, "flowbb", fixture.address.clone()))),
    ]);
    assert!(RecordState::default().mutate(&batch).is_err());
}

trait AdmittedFixture { fn admitted(&self, admission_id: i64, launch_id: i64) -> MutationBatch; }
impl AdmittedFixture for IdentityFixture {
fn admitted(&self, admission_id: i64, launch_id: i64) -> MutationBatch {
    let fixture = self;
    let sender = IdentityFixture::topic("sender");
    let mut launch = fixture.launch(launch_id, "flowaa", fixture.address.clone());
    launch.origin_admission_id_option = Some(OriginAdmissionId(RequestId(admission_id)));
    MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::ArrivalAllocator(ArrivalAllocator { address: fixture.address.clone(), arrival_ordinal: ArrivalOrdinal(1) })),
        Mutation::Put(Record::Queue(Queue { address: fixture.address.clone(), admitted_request_vector: vec![AdmittedRequest { request_id: RequestId(admission_id), address: sender.address, arrival_ordinal: ArrivalOrdinal(0), request: flow_nexus_types::Request::Words("words".to_owned()) }] })),
        Mutation::Put(Record::Permaflow(Permaflow { address: fixture.address.clone(), flow_id_vector: vec![] })),
        Mutation::Put(Record::AddressReservation(fixture.reservation(launch_id, "flowaa", fixture.address.clone()))),
        Mutation::Put(Record::LaunchAttempt(launch)),
    ])
}

}

#[test]
fn distinct_admission_request_links_to_launch() {
    assert!(RecordState::default().mutate(&IdentityFixture::default().admitted(3, 7)).is_ok());
}

#[test]
fn admission_and_launch_request_id_reuse_is_rejected() {
    assert_eq!(RecordState::default().mutate(&IdentityFixture::default().admitted(7, 7)), Err(Invariant::DuplicateRequest(RequestId(7))));
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

#[test]
fn resource_locator_must_match_persisted_creation_intent() {
    let fixture = IdentityFixture::default();
    let mut attempt = fixture.launch(7, "locaaa", fixture.address.clone());
    let mut other_locator = fixture.locator.clone();
    other_locator.correlation_locator = CorrelationLocator("different".into());
    attempt.resource_evidence = ResourceEvidence::NoneConfirmed(other_locator);
    let result = RecordState::default().mutate(&MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::LaunchAttempt(attempt)),
        Mutation::Put(Record::AddressReservation(fixture.reservation(7, "locaaa", fixture.address.clone()))),
    ]));
    assert_eq!(result, Err(Invariant::SnapshotMismatch(
        flow_nexus_types::evidence::RecordKey::LaunchAttempt(RequestId(7)))));
}

#[test]
fn configured_alphabet_is_unique_ascii_alphanumeric_and_has_at_least_two_characters() {
    let fixture = IdentityFixture::default();
    for (alphabet, valid) in [("aB0", true), ("0Z", true), ("", false), ("a", false), ("aa", false), ("a_", false), ("éa", false)] {
        let snapshot = fixture.resolved(fixture.address.clone());
        let configuration = flow_nexus_types::Configuration {
            metaflow_name: flow_nexus_types::MetaflowName("test".into()),
            memory_path: flow_nexus_types::MemoryPath("/memory".into()),
            socket_configuration: flow_nexus_types::SocketConfiguration {
                ordinary_socket_path: flow_nexus_types::OrdinarySocketPath("/ordinary".into()),
                meta_socket_path: flow_nexus_types::MetaSocketPath("/meta".into()),
            },
            declaration_vector: vec![], layer_assignment_vector: vec![],
            model_profile_vector: vec![], harness_profile_vector: vec![], module_source_vector: vec![],
            identity_policy: IdentityPolicy { identity_alphabet: IdentityAlphabet(alphabet.into()) },
            capsule_configuration: snapshot.capsule_configuration,
            security_configuration: flow_nexus_types::SecurityConfiguration { owner_uid: flow_nexus_types::OwnerUid(0) },
        };
        let result = RecordState::default().mutate(&MutationBatch(vec![
            Mutation::Put(Record::ConfigurationRecord(flow_nexus_types::memory::ConfigurationRecord {
                configuration, meta_configure_occurred: flow_nexus_types::evidence::MetaConfigureOccurred(false),
            }))]));
        if valid { assert!(result.is_ok(), "valid alphabet {alphabet}"); }
        else { assert_eq!(result, Err(Invariant::InvalidAlphabet)); }
    }
}

#[test]
fn prepared_phase_cannot_claim_no_resource() {
    let fixture = IdentityFixture::default();
    let mut attempt = fixture.launch(7, "phasea", fixture.address.clone());
    attempt.launch_phase = LaunchPhase::Prepared(flow_nexus_types::evidence::CapsuleHandle {
        creation_locator: fixture.locator.clone(),
        pane_id: flow_nexus_types::evidence::PaneId("pane".into()),
        process_incarnation: fixture.locator.server_binding.process_incarnation.clone(),
    });
    assert_eq!(RecordState::default().mutate(&MutationBatch(vec![
        Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(10) })),
        Mutation::Put(Record::LaunchAttempt(attempt)),
        Mutation::Put(Record::AddressReservation(fixture.reservation(7, "phasea", fixture.address.clone()))),
    ])), Err(Invariant::SnapshotMismatch(flow_nexus_types::evidence::RecordKey::LaunchAttempt(RequestId(7)))));
}
