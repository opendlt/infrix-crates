#[infrix_macros::shapes(default = "alpha")]
pub enum Shape {
    #[infrix_macros::shape(params(rate: f64 = 1))]
    Alpha,
}

fn main() {}
