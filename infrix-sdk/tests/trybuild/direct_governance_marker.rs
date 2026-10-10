use infrix_sdk::governance::AContractMayNotInitiateGovernance;

struct Bypass;

// This must stay impossible outside infrix-sdk. If it compiles, a contract can
// type-check a direct governance wrapper by supplying a local marker value.
impl AContractMayNotInitiateGovernance for Bypass {}

fn main() {}
