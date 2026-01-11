// BAML Generator Configuration
// Uses REST/OpenAPI approach since native Rust codegen isn't available yet
// Run: npx @boundaryml/baml dev --preview
// Server: http://localhost:2024
// Docs: http://localhost:2024/docs

generator target {
  output_dir "../baml_client"
  output_type "rest/openapi"
  version "0.75.0"

  // Generate Rust client via openapi-generator
  openapi_client_type "rust"
}
