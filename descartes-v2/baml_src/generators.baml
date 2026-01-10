// BAML Generator Configuration

generator rust {
  // Output directory for generated Rust code
  output_dir "../src/baml_client"

  // Language target
  language rust

  // Version requirement
  version "^0.70.0"
}

// Alternative: generate to a separate crate
// generator rust_crate {
//   output_dir "../baml_client"
//   language rust
//   version "^0.70.0"
// }
