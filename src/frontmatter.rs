enum MatterType {
  JSON,
  YAML
}

/// Front matter is defined by the block at the top of a document, separated by triple dashes "---"
fn extract_frontmatter(document: &str) -> &str {
  unimplemented!()
}

fn infer_type(matter: &str) -> MatterType {
  unimplemented!()
}