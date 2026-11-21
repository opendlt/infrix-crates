//! The intent goal on the wire, and the priority vocabulary.
//!
//! `IntentGoal` travels as the node's JSON (infrix-schema `intent.IntentGoal`):
//! a declared priority is `goal.priority`, an undeclared one is absent, and the
//! priority words are exactly the node's, read from the vendored
//! `../goal-vocabulary.json` that infrix-core generates from the set its
//! admission validates. infrix-core's SDKGoalParity fence parses the generated
//! enum independently of the generator.

use infrix_types::{AssetAmount, IntentGoal, IntentGoalType, IntentPriority, TargetStateSpec};
use serde_json::{json, Value};

fn vocabulary_priorities() -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../goal-vocabulary.json");
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read the vendored goal vocabulary {path}: {e}"));
    let doc: Value = serde_json::from_str(&raw).expect("goal-vocabulary.json is valid JSON");
    doc["priorities"]
        .as_array()
        .expect("goal-vocabulary.json carries the priorities")
        .iter()
        .map(|p| p.as_str().expect("a priority is a string").to_string())
        .collect()
}

fn call_goal() -> IntentGoal {
    IntentGoal {
        goal_type: IntentGoalType::ContractCall,
        ..IntentGoal::default()
    }
}

#[test]
fn a_goal_with_priority_urgent_encodes_it_as_goal_priority_and_reads_it_back() {
    let goal = IntentGoal {
        priority: Some(IntentPriority::Urgent),
        ..call_goal()
    };
    let wire = serde_json::to_value(json!({ "goal": goal })).expect("serialisable");
    assert_eq!(wire["goal"]["priority"], json!("urgent"));
    assert_eq!(wire["goal"]["type"], json!("CONTRACT_CALL"));
    let back: IntentGoal = serde_json::from_value(wire["goal"].clone()).expect("the node's form reads back");
    assert_eq!(back, goal);
}

#[test]
fn an_omitted_priority_is_absent_on_the_wire() {
    let wire = serde_json::to_value(call_goal()).expect("serialisable");
    let obj = wire.as_object().expect("a goal is an object");
    assert!(!obj.contains_key("priority"), "an undeclared priority must not be written: {wire}");
    // The node's omitempty fields are absent too; sourceAssets is always written, as the node writes it.
    assert_eq!(
        obj.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["sourceAssets", "type"]
    );
    let back: IntentGoal = serde_json::from_value(wire).expect("reads back");
    assert_eq!(back.priority, None);
}

#[test]
fn the_nodes_goal_json_reads_with_every_field_named_as_the_node_names_it() {
    // The shape infrix-schema's IntentGoal marshals: a nil sourceAssets is null.
    let node = json!({
        "type": "SETTLEMENT",
        "sourceAssets": null,
        "targetAssets": [{"asset": "acc://a.acme/tokens", "amount": 5, "amountDecimal": "5.0", "isMinimum": true,
                          "tokenStandard": "ACME", "contractUrl": "acc://a.acme/c"}],
        "targetState": {"stateType": "balance", "parameters": {"min": "5"}, "contract": "acc://a.acme/c"},
        "via": "dex",
        "customType": "x",
        "priority": "high"
    });
    let goal: IntentGoal = serde_json::from_value(node.clone()).expect("the node's goal reads");
    assert_eq!(goal.goal_type, IntentGoalType::Settlement);
    assert!(goal.source_assets.is_empty());
    assert_eq!(goal.priority, Some(IntentPriority::High));
    assert_eq!(
        goal.target_assets,
        vec![AssetAmount {
            asset: "acc://a.acme/tokens".into(),
            amount: 5,
            amount_decimal: Some("5.0".into()),
            is_minimum: true,
            is_maximum: false,
            token_standard: Some("ACME".into()),
            contract_url: Some("acc://a.acme/c".into()),
        }]
    );
    assert_eq!(
        goal.target_state,
        Some(TargetStateSpec {
            state_type: "balance".into(),
            parameters: vec![("min".into(), "5".into())],
            contract: Some("acc://a.acme/c".into()),
        })
    );
    // Written back, it is the node's form (sourceAssets as an empty list, isMaximum=false omitted).
    let mut want = node;
    want["sourceAssets"] = json!([]);
    assert_eq!(serde_json::to_value(&goal).expect("serialisable"), want);
}

#[test]
fn a_priority_or_goal_type_the_node_refuses_is_refused_here() {
    for bad in ["URGENT", " urgent", "critical", ""] {
        let wire = json!({"type": "CONTRACT_CALL", "sourceAssets": [], "priority": bad});
        assert!(
            serde_json::from_value::<IntentGoal>(wire).is_err(),
            "priority {bad:?} was read, and the node refuses it"
        );
    }
    let wire = json!({"type": "SEND_TOKENS", "sourceAssets": []});
    assert!(serde_json::from_value::<IntentGoal>(wire).is_err(), "SEND_TOKENS is not a goal");
    let dup = r#"{"type":"CUSTOM","sourceAssets":[],"targetState":{"stateType":"s","parameters":{"k":"1","k":"2"}}}"#;
    assert!(serde_json::from_str::<IntentGoal>(dup).is_err(), "a parameter key given twice was resolved silently");
}

#[test]
fn intent_priority_all_is_exactly_the_node_vocabulary() {
    let sdk: Vec<String> = IntentPriority::ALL.iter().map(|p| p.as_str().to_string()).collect();
    assert_eq!(sdk, vocabulary_priorities());
}

#[test]
fn every_priority_serialises_as_its_as_str_and_round_trips() {
    for p in IntentPriority::ALL.iter() {
        assert_eq!(serde_json::to_value(p).expect("serialisable"), json!(p.as_str()));
        assert_eq!(IntentPriority::from_wire(p.as_str()).as_ref(), Some(p));
        let back: IntentPriority = serde_json::from_value(json!(p.as_str())).expect("reads back");
        assert_eq!(&back, p);
    }
    for bad in ["URGENT", "Urgent", " normal", "critical", ""] {
        assert_eq!(IntentPriority::from_wire(bad), None, "{bad:?} must be refused");
    }
}
