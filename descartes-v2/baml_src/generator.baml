// BAML Generator Configuration
// =============================
// Native Rust codegen - compiles BAML to Rust at build time
//
// Regenerate after modifying .baml files:
//   npx @boundaryml/baml generate --from baml_src
//
// Generated code location: baml_client/baml_client/
// Usage in Rust:
//   use crate::baml_client::async_client::B;
//   let result = B.DecideNextAction.call(args).await?;
//
// No server required - BAML compiles directly to Rust types

generator target {
  output_dir "../baml_client"
  output_type "rust"
  version "0.217.0"
}
