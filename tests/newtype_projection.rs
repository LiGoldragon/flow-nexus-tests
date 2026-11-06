#![cfg(feature = "datom")]

#[path = "fixtures/newtype.rs"]
mod newtype;

use datom_codec::{Actualizing, Budget, Datomizable, Potential};
use newtype::{Address, Topic};
use protos::{Protosizable, ReaderBudget, Textualizable};

#[test]
fn transparent_topic_writer_is_bare_scalar() {
    let value = Topic("flowNexus".to_owned());
    let actual = value.datomize(vec![]).protosize().textualize();
    eprintln!("Topic writer output: {actual}");
    assert_eq!(actual, "flowNexus");
}

#[test]
fn parser_accepts_transparent_topic() {
    let mut topic_budget = Budget {
        remaining: 1 << 20,
        reader: ReaderBudget::try_from(1 << 20).expect("bounded reader budget"),
        depth: 0,
        maximum_depth: 256,
    };
    let topic = Potential::<Topic>::from("flowNexus".to_owned()).actualize(&mut topic_budget);
    eprintln!("Topic parser result for bare scalar: {topic:?}");
    assert_eq!(topic.unwrap(), Topic("flowNexus".to_owned()));
}

#[test]
fn parser_accepts_transparent_address_position() {
    let mut address_budget = Budget {
        remaining: 1 << 20,
        reader: ReaderBudget::try_from(1 << 20).expect("bounded reader budget"),
        depth: 0,
        maximum_depth: 256,
    };
    let address = Potential::<Address>::from("{ flowNexus }".to_owned()).actualize(&mut address_budget);
    eprintln!("Address parser result for scalar position: {address:?}");
    assert_eq!(address.unwrap(), Address { topic: Topic("flowNexus".to_owned()) });
}
