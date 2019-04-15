pub fn render_markdown(contents: &str) -> String {
  comrak::markdown_to_html(contents, &comrak::ComrakOptions {
      hardbreaks: true,
      smart: true,
      github_pre_lang: true,
      unsafe_: true,
      width: 80,
      ext_strikethrough: true,
      ext_tagfilter: false,
      ext_table: true,
      ext_autolink: true,
      ext_tasklist: true,
      ext_superscript: true,
      ext_header_ids: Some("h-".to_string()),
      ext_footnotes: true,
      ext_description_lists: true,
      ..comrak::ComrakOptions::default()
  })
}