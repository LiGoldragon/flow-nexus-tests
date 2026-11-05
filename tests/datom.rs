#![cfg(feature = "datom")]
use datom_codec::{Actualizing, Budget, Datomizable, Potential};
use flow_nexus_types::{Address, Aspect, Brief, Declaration, FlowId, HarnessModuleName, Layer, Remembering, Request, Topic};
use flow_nexus_types::evidence::{ArrivalOrdinal, Mutation, MutationBatch, ReadSelector, Record, RecordKey, RequestId};
use flow_nexus_types::memory::{ArrivalAllocator, RequestAllocator};
use meta_signal_flow_nexus::{Query as MetaQuery, Response as MetaResponse, Refused_Data as MetaRefused};
use protos::{Compactable, Protosizable, ReaderBudget};
use signal_flow_nexus::{Launch_Data, Query, Send_Data, Response as OrdinaryResponse, Refused_Data as OrdinaryRefused};

struct FixtureBudget { budget: Budget }
impl Default for FixtureBudget {
    fn default() -> Self {
        Self { budget: Budget { remaining: 1 << 20, reader: ReaderBudget { remaining: 1 << 20 }, depth: 0, maximum_depth: 256 } }
    }
}
trait Witnessing {
    fn witness<T: Datomizable + datom_codec::Composing + PartialEq + std::fmt::Debug>(&mut self, text: &str, expected: T);
}
impl Witnessing for FixtureBudget {
    // The text and the value are written independently: decode must give the value, and the value must print the text.
    fn witness<T: Datomizable + datom_codec::Composing + PartialEq + std::fmt::Debug>(&mut self, text: &str, expected: T) {
        let decoded = Potential::<T>::from(text.to_owned()).actualize(&mut self.budget).unwrap_or_else(|error| panic!("{text}: {error:?}"));
        assert_eq!(decoded, expected, "{text}");
        assert_eq!(expected.canonical(), text);
    }
}
fn address() -> Address {
    Address { aspect: Aspect::Mind, topic: Topic("flowNexus".to_owned()), layer: Layer::Tertiary }
}
trait Canonical { fn canonical(&self) -> String; }
impl<T: Datomizable> Canonical for T {
    fn canonical(&self) -> String { self.datomize(vec![]).protosize().compact() }
}

#[test]
fn ordinary_response_roundtrips_in_its_typed_vocabulary() {
    let mut fixture = FixtureBudget::default();
    for value in [OrdinaryResponse::Ended, OrdinaryResponse::Delivered, OrdinaryResponse::Queued, OrdinaryResponse::Recorded] {
        let line = value.canonical();
        let decoded = Potential::<OrdinaryResponse>::from(line.clone()).actualize(&mut fixture.budget).expect("ordinary response");
        assert_eq!(decoded, value);
        assert_eq!(decoded.canonical(), line);
    }
}

#[test]
fn meta_response_roundtrips_in_its_typed_vocabulary() {
    let mut fixture = FixtureBudget::default();
    for value in [MetaResponse::Configured, MetaResponse::Declared, MetaResponse::Thresholded] {
        let line = value.canonical();
        let decoded = Potential::<MetaResponse>::from(line.clone()).actualize(&mut fixture.budget).expect("meta response");
        assert_eq!(decoded, value);
        assert_eq!(decoded.canonical(), line);
    }
}

#[test]
fn declared_input_positions_match_independent_expected_fields() {
    let mut fixture = FixtureBudget::default();
    let text = "Launch.{ { Mind flowNexus Tertiary } «» «Report that this test flow started.» }";
    let actual = Potential::<Query>::from(text.to_owned()).actualize(&mut fixture.budget).expect("declared Launch");
    let expected = Query::Launch(Launch_Data {
        address: Address { aspect: Aspect::Mind, topic: Topic("flowNexus".to_owned()), layer: Layer::Tertiary },
        remembering: Remembering(String::new()),
        brief: Brief("Report that this test flow started.".to_owned()),
    });
    assert_eq!(actual, expected);
    let actual = Potential::<Query>::from("End.a7b8c9".to_owned()).actualize(&mut fixture.budget).expect("End");
    assert_eq!(actual, Query::End(FlowId("a7b8c9".to_owned())));
    assert_eq!(actual.canonical(), "End.a7b8c9");
    let actual = Potential::<OrdinaryResponse>::from("Refused.UnknownFlow".to_owned()).actualize(&mut fixture.budget).expect("typed refusal");
    assert_eq!(actual, OrdinaryResponse::Refused(OrdinaryRefused::UnknownFlow));
    assert_eq!(actual.canonical(), "Refused.UnknownFlow");
}

#[test]
fn malformed_wrong_arity_and_wrong_vocabulary_are_rejected() {
    let mut fixture = FixtureBudget::default();
    for text in ["End", "End.{ a7b8c9 extra }", "Launch.{ { Mind flowNexus Tertiary } «» }", "Launch.{", "Refused.UnknownFlow"] {
        assert!(Potential::<Query>::from(text.to_owned()).actualize(&mut fixture.budget).is_err(), "unexpected accepted input {text}");
    }
    let ordinary = OrdinaryResponse::Refused(OrdinaryRefused::UnknownFlow).canonical();
    assert!(Potential::<MetaResponse>::from(ordinary).actualize(&mut fixture.budget).is_err());
    let meta = MetaResponse::Refused(MetaRefused::NotOwner).canonical();
    assert!(Potential::<OrdinaryResponse>::from(meta).actualize(&mut fixture.budget).is_err());
}

#[test]
fn every_request_variant_crosses_the_ordinary_send() {
    let mut fixture = FixtureBudget::default();
    for (text, request) in [
        ("Send.{ { Mind flowNexus Tertiary } Order.«Review the fix.» }", Request::Order("Review the fix.".to_owned())),
        ("Send.{ { Mind flowNexus Tertiary } Question.«Which revision?» }", Request::Question("Which revision?".to_owned())),
        ("Send.{ { Mind flowNexus Tertiary } Words.«Fix it today.» }", Request::Words("Fix it today.".to_owned())),
        ("Send.{ { Mind flowNexus Tertiary } Result.«Reviewed and green.» }", Request::Result("Reviewed and green.".to_owned())),
        ("Send.{ { Mind flowNexus Tertiary } Notice.«The build is green.» }", Request::Notice("The build is green.".to_owned())),
    ] {
        fixture.witness(text, Query::Send(Send_Data { address: address(), request }));
    }
}

#[test]
fn declaration_carries_its_harness_module_names() {
    let mut fixture = FixtureBudget::default();
    fixture.witness(
        "Declare.{ { Mind flowNexus Tertiary } [] [ Artifact ClaudeMd ] }",
        MetaQuery::Declare(Declaration {
            address: address(),
            module_id_vector: vec![],
            harness_module_name_vector: vec![HarnessModuleName("Artifact".to_owned()), HarnessModuleName("ClaudeMd".to_owned())],
        }),
    );
}

#[test]
fn owner_memory_surface_crosses_the_meta_wire() {
    let mut fixture = FixtureBudget::default();
    fixture.witness(
        "Mutate.[ Put.RequestAllocator.{ 8 } Remove.Flow.a7b8c9 ]",
        MetaQuery::Mutate(MutationBatch(vec![
            Mutation::Put(Record::RequestAllocator(RequestAllocator { request_id: RequestId(8) })),
            Mutation::Remove(RecordKey::Flow(FlowId("a7b8c9".to_owned()))),
        ])),
    );
    fixture.witness("Read.All", MetaQuery::Read(ReadSelector::All));
    fixture.witness("Read.ByKey.RequestAllocatorSingleton", MetaQuery::Read(ReadSelector::ByKey(RecordKey::RequestAllocatorSingleton)));
    fixture.witness(
        "ReadRecords.[ RequestAllocator.{ 8 } ArrivalAllocator.{ { Mind flowNexus Tertiary } 3 } ]",
        MetaResponse::ReadRecords(vec![
            Record::RequestAllocator(RequestAllocator { request_id: RequestId(8) }),
            Record::ArrivalAllocator(ArrivalAllocator { address: address(), arrival_ordinal: ArrivalOrdinal(3) }),
        ]),
    );
}
