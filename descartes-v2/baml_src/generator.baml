// BAML Generator Configuration
// Generates typed client code from .baml files

generator target {
  // Output to src/baml_client for Rust integration
  output_dir "../src/baml_client"
  output_type "rust"
  version "0.75.0"
}
