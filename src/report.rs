use crate::analyzer::HarborReport;
use crate::model::{Finding, FindingSeverity};

pub fn render_text(report: &HarborReport) -> String {
    let mut out = String::new();
    out.push_str("PortWatch report\n");
    out.push_str("================\n");
    out.push_str(&format!("stream: {}\n", report.stream_id));
    out.push_str(&format!("events: {}\n", report.event_count));
    out.push_str(&format!("risk: {}\n", report.risk_score));
    out.push_str(&format!("frames: {}\n", report.stats.frames));
    out.push_str(&format!("sections: {}\n", report.stats.sections));
    out.push_str(&format!(
        "compression ratio: {:.2}\n",
        report.stats.compression_ratio()
    ));
    out.push_str("\nfindings\n");
    for finding in &report.findings {
        out.push_str(&render_finding(finding));
        out.push('\n');
    }
    out
}

pub fn render_finding(finding: &Finding) -> String {
    let vessel = finding
        .vessel
        .map(|id| id.to_string())
        .unwrap_or_else(|| "-".to_owned());
    format!(
        "[{}] {} vessel={} {}",
        severity_label(&finding.severity),
        finding.code,
        vessel,
        finding.detail
    )
}

pub fn severity_label(severity: &FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Info => "info",
        FindingSeverity::Low => "low",
        FindingSeverity::Medium => "medium",
        FindingSeverity::High => "high",
        FindingSeverity::Critical => "critical",
    }
}

pub fn render_jsonish(report: &HarborReport) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(&format!("\"stream_id\":{},", report.stream_id));
    out.push_str(&format!("\"event_count\":{},", report.event_count));
    out.push_str(&format!("\"risk_score\":{},", report.risk_score));
    out.push_str("\"findings\":[");
    for (idx, finding) in report.findings.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str(&format!("\"severity\":\"{}\",", severity_label(&finding.severity)));
        out.push_str(&format!("\"code\":\"{}\",", escape(finding.code)));
        if let Some(vessel) = finding.vessel {
            out.push_str(&format!("\"vessel\":\"{}\",", vessel));
        } else {
            out.push_str("\"vessel\":null,");
        }
        out.push_str(&format!("\"detail\":\"{}\"", escape(&finding.detail)));
        out.push('}');
    }
    out.push_str("]}");
    out
}

pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

pub fn top_findings(report: &HarborReport, limit: usize) -> Vec<&Finding> {
    let mut findings: Vec<&Finding> = report.findings.iter().collect();
    findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity.weight()));
    findings.truncate(limit);
    findings
}
