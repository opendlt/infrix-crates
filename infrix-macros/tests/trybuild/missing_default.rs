#[infrix_macros::shapes]
pub enum Shape {
    #[infrix_macros::shape(params(rate: u64 = 1))]
    Alpha,
}

fn main() {}
