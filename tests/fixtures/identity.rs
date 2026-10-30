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

pub(crate) struct IdentityFixture {
    pub(crate) address: Address,
    pub(crate) locator: CreationLocator,
}

pub(crate) trait FixtureFactory {
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
            identity_policy: IdentityPolicy { identity_alphabet: IdentityAlphabet("abcdefghijklmnopqrstuvwxyz".to_owned()) },
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

