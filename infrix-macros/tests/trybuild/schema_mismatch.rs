#[infrix_macros::shapes(default = "alpha")]
pub enum Shape {
    #[infrix_macros::shape(params(rate: u64 = 1))]
    Alpha,

    #[infrix_macros::shape(params(enabled: bool = true))]
    Beta,
}

fn main() {}
