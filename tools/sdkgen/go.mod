// sdkgen keeps this repo's Rust IntentGoalType enum in lock-step with the
// published infrix-schema contract module. It is a dev/CI-only tool module (the
// crates themselves are pure Rust).
module github.com/opendlt/infrix-crates/tools/sdkgen

go 1.25.7

require github.com/opendlt/infrix-schema v0.2.0
