#![cfg(feature = "datom")]
use datom_codec::{Actualizing, Budget, Datomizable, Potential};
use flow_nexus_types::{Address, Aspect, Brief, FlowId, Layer, Remembering, Topic};
use meta_signal_flow_nexus::{Response as MetaResponse, Refused_Data as MetaRefused};
use protos::{Compactable, Protosizable, ReaderBudget};
use signal_flow_nexus::{Launch_Data, Query, Response as OrdinaryResponse, Refused_Data as OrdinaryRefused};

struct FixtureBudget { budget: Budget }
impl Default for FixtureBudget {
    fn default() -> Self {
        Self { budget: Budget { remaining: 1 << 20, reader: ReaderBudget { remaining: 1 << 20 }, depth: 0, maximum_depth: 256 } }
    }
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
fn flow_id_newtype_matches_the_supplied_scalar_cli_form() {
    let mut fixture = FixtureBudget::default();
    let actual = Potential::<Query>::from("End.a7b8c9".to_owned()).actualize(&mut fixture.budget).expect("supplied scalar FlowId form");
    assert_eq!(actual, Query::End(FlowId("a7b8c9".to_owned())));
}

#[test]
fn refusal_without_newtype_payload_has_the_expected_form() {
    let mut fixture = FixtureBudget::default();
    let expected = OrdinaryResponse::Refused(OrdinaryRefused::UnknownFlow);
    let actual = Potential::<OrdinaryResponse>::from("Refused.UnknownFlow".to_owned()).actualize(&mut fixture.budget).expect("declared refusal form");
    assert_eq!(actual, expected);
    assert_eq!(actual.canonical(), "Refused.UnknownFlow");
}
