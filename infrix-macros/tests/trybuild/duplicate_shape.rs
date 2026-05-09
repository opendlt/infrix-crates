#[infrix_macros::shapes(default = "alpha")]
pub enum Shape {
    #[infrix_macros::shape(name = "alpha", params(rate: u64 = 1))]
    Alpha,

    #[infrix_macros::shape(name = "alpha", params(rate: u64 = 2))]
    AlphaAgain,
}

fn main() {}
