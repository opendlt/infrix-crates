#[infrix_macros::evolution_rules]
pub mod rules {
    #[infrix_macros::rule(target = "Growth", condition = "env::price() > 200")]
    pub fn grow() {}
}

fn main() {}
