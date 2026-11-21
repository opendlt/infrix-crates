//! The intent goal on the wire, and the priority vocabulary.
//!
//! `IntentGoal` travels as the node's JSON (infrix-schema `intent.IntentGoal`):
//! a declared priority is `goal.priority`, an undeclared one is absent, and the
//! priority words are exactly the node's, read from the vendored
//! `../goal-vocabulary.json` that infrix-core generates from the set its
//! admission validates. infrix-core's SDKGoalParity fence parses the generated
//! enum independently of the generator.

use infrix_types::{
    AssetAmount, IntentGoal, IntentGoalType, IntentPriority, IntentSubmitRequest, TargetStateSpec,
};
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
    let back: IntentGoal =
        serde_json::from_value(wire["goal"].clone()).expect("the node's form reads back");
    assert_eq!(back, goal);
}

#[test]
fn an_omitted_priority_is_absent_on_the_wire() {
    let wire = serde_json::to_value(call_goal()).expect("serialisable");
    let obj = wire.as_object().expect("a goal is an object");
    assert!(
        !obj.contains_key("priority"),
        "an undeclared priority must not be written: {wire}"
    );
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
    assert!(
        serde_json::from_value::<IntentGoal>(wire).is_err(),
        "SEND_TOKENS is not a goal"
    );
    let dup = r#"{"type":"CUSTOM","sourceAssets":[],"targetState":{"stateType":"s","parameters":{"k":"1","k":"2"}}}"#;
    assert!(
        serde_json::from_str::<IntentGoal>(dup).is_err(),
        "a parameter key given twice was resolved silently"
    );
}

#[test]
fn intent_priority_all_is_exactly_the_node_vocabulary() {
    let sdk: Vec<String> = IntentPriority::ALL
        .iter()
        .map(|p| p.as_str().to_string())
        .collect();
    assert_eq!(sdk, vocabulary_priorities());
}

#[test]
fn every_priority_serialises_as_its_as_str_and_round_trips() {
    for p in IntentPriority::ALL.iter() {
        assert_eq!(
            serde_json::to_value(p).expect("serialisable"),
            json!(p.as_str())
        );
        assert_eq!(IntentPriority::from_wire(p.as_str()).as_ref(), Some(p));
        let back: IntentPriority = serde_json::from_value(json!(p.as_str())).expect("reads back");
        assert_eq!(&back, p);
    }
    for bad in ["URGENT", "Urgent", " normal", "critical", ""] {
        assert_eq!(
            IntentPriority::from_wire(bad),
            None,
            "{bad:?} must be refused"
        );
    }
}

// ---- item 2c part 28aa: customParams and the governed-submit contract ----

#[test]
fn a_node_goals_custom_params_are_read_and_written_back() {
    let node = json!({
        "type": "CONTRACT_CALL",
        "sourceAssets": null,
        "customParams": { "contract": "acc://alice.acme/counter", "function": "increment", "gasLimit": 200000, "nested": { "a": [1, "b"] } }
    });
    let goal: IntentGoal = serde_json::from_value(node.clone()).expect("the node's goal reads");
    let params = goal
        .custom_params
        .as_ref()
        .expect("customParams is carried, never ignored");
    assert_eq!(params["function"], json!("increment"));
    assert_eq!(params["nested"]["a"][1], json!("b"));
    let back = serde_json::to_value(&goal).expect("serialisable");
    assert_eq!(back["customParams"], node["customParams"]);
    let plain = serde_json::to_value(call_goal()).expect("serialisable");
    assert!(
        !plain.as_object().unwrap().contains_key("customParams"),
        "an absent customParams must not be written"
    );
}

/// The request the TS client sends for a fully populated goal and options (infrix-sdk-js typescript/src/submitContract.test.ts
/// FULL_SDK_SUBMISSION, decoded by infrix-core's TestS28aaTheSDKWireDecodesIntoTheContract): the Rust request writes exactly it.
fn full_sdk_submission() -> serde_json::Value {
    json!({
        "goalType": "SWAP",
        "actor": "acc://alice.acme",
        "purpose": "operational",
        "workflowInstance": "wf-28aa",
        "sourceAssets": [{ "asset": "acc://acme/ACME", "amount": 1000, "amountDecimal": "10.00", "isMinimum": true, "tokenStandard": "acme", "contractUrl": "acc://acme/ACME" }],
        "targetAssets": [{ "asset": "acc://usd.acme/USD", "amount": 950, "amountDecimal": "9.50", "isMaximum": true }],
        "targetState": { "stateType": "token_account", "parameters": { "owner": "acc://alice.acme" }, "contract": "acc://c.acme" },
        "via": "acc://dex.acme",
        "customType": "label",
        "customParams": { "slippage": 0.01 },
        "priority": "urgent",
        "constraints": { "minOutput": 900, "minOutputDecimal": "9.00", "maxGas": 50000, "maxCredits": 7, "minConfidence": 0.5, "minAvgConfidence": 0.6, "maxSteps": 9, "maxSlippage": 0.02, "requiredContracts": ["acc://dex.acme"], "excludedContracts": ["acc://bad.acme"], "deadline": 4102444800u64, "allowedImmuneStates": ["quarantined"] },
        "preferences": { "optimize": "minimize_cost", "customWeights": { "cost": 0.7 }, "preferContracts": ["acc://dex.acme"], "avoidContracts": ["acc://slow.acme"], "maxAlternatives": 3 },
        "userAddress": "acc://alice.acme",
        "rawInput": "swap 10 ACME for at least 9 USD",
        "expiresAt": 5000,
        "preview": true
    })
}

#[test]
fn the_submit_request_round_trips_the_full_governed_submit_contract() {
    let wire = full_sdk_submission();
    let req: IntentSubmitRequest =
        serde_json::from_value(wire.clone()).expect("the contract reads into the Rust request");
    assert_eq!(req.goal_type, IntentGoalType::Swap);
    assert_eq!(req.priority, Some(IntentPriority::Urgent));
    assert_eq!(
        req.constraints.as_ref().unwrap().min_average_confidence,
        0.6
    );
    assert_eq!(req.expires_at, Some(5000));
    let back = serde_json::to_value(&req).expect("serialisable");
    assert_eq!(
        back, wire,
        "the Rust request must write exactly the contract"
    );
    // the goal it states, and the request built back from that goal, carry every goal member
    let goal = req.goal();
    let rebuilt = IntentSubmitRequest::from_goal(goal.clone(), req.actor.clone());
    assert_eq!(rebuilt.goal(), goal);
    assert_eq!(
        serde_json::to_value(&goal).unwrap()["customParams"],
        json!({ "slippage": 0.01 })
    );
}

#[test]
fn a_member_the_contract_does_not_name_is_refused_by_name() {
    for (member, value) in [
        ("metadata", json!({ "a": "b" })),
        ("publicKey", json!("aa")),
        ("goal", json!({})),
    ] {
        let mut wire = full_sdk_submission();
        wire.as_object_mut()
            .unwrap()
            .insert(member.to_string(), value);
        let err = serde_json::from_value::<IntentSubmitRequest>(wire)
            .expect_err("an unknown member must be refused");
        assert!(err.to_string().contains(member), "{member}: {err}");
    }
    let mut nested = full_sdk_submission();
    nested["constraints"]
        .as_object_mut()
        .unwrap()
        .insert("maxGass".to_string(), json!(1));
    assert!(
        serde_json::from_value::<IntentSubmitRequest>(nested).is_err(),
        "an unknown constraint must be refused"
    );
}
