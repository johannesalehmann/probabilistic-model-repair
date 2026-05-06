use prism_parser::Span;

#[derive(Clone)]
pub struct RepairedSpan {
    section: Span,
    replace_by: String,
    kind: RepairKind,
}

impl RepairedSpan {
    pub fn new(section: Span, replace_by: String, kind: RepairKind) -> Self {
        Self {
            section,
            replace_by,
            kind,
        }
    }
}

#[derive(Clone)]
pub enum RepairKind {
    Fix,
    Unchanged,
    ToRepair,
}

enum RepairTitle {
    Base,
    Repair,
}

pub struct Repair {
    spans: Vec<RepairedSpan>,
    cost: f64,
    title: RepairTitle,
}

impl Repair {
    pub fn new_base() -> Self {
        Self {
            spans: Vec::new(),
            cost: 0.0,
            title: RepairTitle::Base,
        }
    }

    pub fn new_repair(cost: f64) -> Self {
        Self {
            spans: Vec::new(),
            cost,
            title: RepairTitle::Repair,
        }
    }

    pub fn add_span(&mut self, span: RepairedSpan) {
        self.spans.push(span);
    }
}

pub struct RepairOutput {
    base_source_code: String,
    repairs: Vec<Repair>,
}

impl RepairOutput {
    pub fn new(base_source_code: String) -> Self {
        Self {
            base_source_code,
            repairs: Vec::new(),
        }
    }

    pub fn add_repair(&mut self, repair: Repair) {
        self.repairs.push(repair);
    }

    pub fn to_html(&self) -> String {
        let base = include_str!("html_template.html");

        let mut tab_headers = Vec::new();
        let mut tabs = Vec::new();

        for repair in &self.repairs {
            let mut entries = repair.spans.clone();
            entries.sort_unstable_by(|e1, e2| e2.section.start.cmp(&e1.section.start));

            // We need to replace < by &lt; and > by &gt; but if we do it now, we break the indexing
            //  used during replacement. Instead, use arbitrary temporary characters and replace them
            //  later.
            let mut code = self.base_source_code.replace("<", "\\").replace(">", "~");
            for repair_entry in entries {
                let class = match repair_entry.kind {
                    RepairKind::Fix => "repair",
                    RepairKind::Unchanged => "",
                    RepairKind::ToRepair => "to-repair",
                };
                code.replace_range(
                    repair_entry.section.into_range(),
                    &format!(
                        "<span class=\"{}\">{}</span>",
                        class, repair_entry.replace_by
                    ),
                )
            }

            let title = match repair.title {
                RepairTitle::Base => "base".to_string(),
                RepairTitle::Repair => {
                    format!("repair (cost: {:.2})", repair.cost)
                }
            };
            tab_headers.push(title);
            code = code.replace("\\", "&lt;").replace("~", "&gt;");
            tabs.push(Self::code_to_html(&code));
        }

        let tab_header_string = tab_headers.iter().enumerate().map(|(i, h)|{
            let active = match i {
                0 => " active",
                _ => "",
            };
            format!("<span class=\"repair-tab{}\" onclick=\"show({});\" id=\"tab-header-{}\">{}</span>", active, i, i, h)}
        ).collect::<Vec<_>>().join("\n");

        let tabs_string = tabs
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let visible = match i {
                    0 => "",
                    _ => " style=\"display: none;\"",
                };
                format!(
                    "<div class=\"tab\" id=\"tab-{}\"{}>\n{}\n</div>",
                    i, visible, h
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        base.replace("<!-- insert tab headers here -->", &tab_header_string)
            .replace("<!-- insert tabs here -->", &tabs_string)
    }

    fn code_to_html(base: &str) -> String {
        let mut lines = Vec::new();
        for line in base.lines() {
            let mut chars = Vec::new();
            let mut inside_tag = false;
            for char in line.chars() {
                if char == '<' {
                    inside_tag = true;
                } else if char == '>' {
                    inside_tag = false;
                }
                if char == ' ' && !inside_tag {
                    chars.extend_from_slice(&['&', 'n', 'b', 's', 'p', ';']);
                } else {
                    chars.push(char);
                }
            }
            let mut with_nbsp = chars.into_iter().collect::<String>();
            if with_nbsp.is_empty() {
                with_nbsp = "&nbsp;".to_string();
            }
            lines.push(format!("<p>{}</p>", with_nbsp));
        }

        lines.join("\n")
    }
}
