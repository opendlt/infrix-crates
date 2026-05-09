extern crate self as infrix_sdk;

pub mod shapes {
    pub fn param_u64(name: &str) -> u64 {
        match name {
            "ltv_ratio" => 5000,
            _ => panic!("unexpected u64 param {name}"),
        }
    }

    pub fn param_i64(name: &str) -> i64 {
        panic!("unexpected i64 param {name}")
    }

    pub fn param_bool(name: &str) -> bool {
        match name {
            "borrow_enabled" => true,
            _ => panic!("unexpected bool param {name}"),
        }
    }

    pub fn param_string(name: &str) -> String {
        panic!("unexpected string param {name}")
    }

    pub fn param_bytes(name: &str) -> Vec<u8> {
        panic!("unexpected bytes param {name}")
    }

    pub fn param_u128(name: &str) -> u128 {
        panic!("unexpected u128 param {name}")
    }
}

#[infrix_macros::shapes(
    contract = "Lending",
    default = "conservative",
    evaluation_interval = 2,
    max_transitions_per_day = 3
)]
pub enum Shape {
    #[infrix_macros::shape(
        description = "Low-risk parameters",
        color = "green",
        params(ltv_ratio: u64 = 5000, borrow_enabled: bool = true),
    )]
    Conservative,

    #[infrix_macros::shape(
        description = "Higher-growth parameters",
        color = "blue",
        params(ltv_ratio: u64 = 7500, borrow_enabled: bool = true),
    )]
    Growth,
}

#[infrix_macros::evolution_rules(cooldown_blocks = 5)]
pub mod rules {
    #[infrix_macros::rule(
        priority = 0,
        target = "growth",
        source = "conservative",
        condition = r#"env::price() > 200"#,
        duration_blocks = 7
    )]
    pub fn grow_on_price() {}
}

#[test]
fn shapes_macro_emits_deterministic_runtime_json_and_accessors() {
    let json = Shape::__shape_json();
    assert!(json.contains("\"contract_name\":\"Lending\""));
    assert!(json.contains("\"default_shape\":\"conservative\""));
    assert!(json.contains("\"name\":\"growth\""));
    assert!(json.contains("\"parameter_schema\":[{\"name\":\"ltv_ratio\",\"type\":\"u64\"},{\"name\":\"borrow_enabled\",\"type\":\"bool\"}]"));
    assert!(json.contains("\"evaluation_interval\":2"));
    assert!(json.contains("\"max_transitions_per_day\":3"));

    assert_eq!(Shape::ltv_ratio(), 5000);
    assert!(Shape::borrow_enabled());
}

#[test]
fn evolution_rules_macro_emits_deterministic_rule_fragment() {
    assert!(rules::__INFRIX_SHAPE_RULES_JSON.contains("\"evolution_rules\""));
    assert!(rules::__INFRIX_SHAPE_RULES_JSON.contains("\"target_shape\":\"growth\""));
    assert!(rules::__INFRIX_SHAPE_RULES_JSON.contains("\"source_shapes\":[\"conservative\"]"));
    assert!(rules::__INFRIX_SHAPE_RULES_JSON.contains("\"duration_blocks\":7"));
    assert!(rules::__INFRIX_SHAPE_RULES_JSON.contains("\"cooldown_blocks\":5"));
}
