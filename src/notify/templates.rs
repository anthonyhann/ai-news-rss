use std::collections::HashSet;

use serde_json::{Value, json};

use crate::models::{ActionOption, Brief, TopSignal};

pub fn digest_subject(brief: &Brief) -> String {
    format!("Po Radar · {} · {}", slot_label(brief), brief.date)
}

pub fn feishu_digest_card(brief: &Brief) -> Value {
    let signals = signals(brief);
    let lead = signals.first().copied();
    let quick = signals.iter().skip(1).take(2).copied().collect::<Vec<_>>();
    let summary = executive_summary(brief, &signals);
    let brief_url = public_brief_url();

    let mut elements = vec![
        json!({
            "tag": "markdown",
            "content": format!(
                "**{}** {}\n\n{}",
                greeting(brief),
                intro(brief, signals.len()),
                sources_line(brief)
            )
        }),
        json!({"tag": "hr"}),
        json!({
            "tag": "markdown",
            "content": format!("**{}**\n{}", section_label(brief, "60 秒版", "60-SECOND BRIEF"), summary)
        }),
    ];

    if let Some(item) = lead {
        elements.push(json!({"tag": "hr"}));
        elements.push(json!({
            "tag": "markdown",
            "content": format!(
                "**{}**\n### [{}]({}) · **{:.0} {}**\n\n{}\n\n**{}** {}",
                section_label(brief, "今日头条", "TODAY'S LEAD"),
                title(item, brief),
                item.url,
                item.score,
                score_unit(brief),
                summary_text(item, brief),
                section_label(brief, "为什么重要", "WHY IT MATTERS"),
                why(item, brief)
            )
        }));
    }

    if !quick.is_empty() {
        let rows = quick
            .iter()
            .enumerate()
            .map(|(index, item)| {
                format!(
                    "{:02} · [{}]({}) · **{:.0}**",
                    index + 2,
                    title(item, brief),
                    item.url,
                    item.score
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        elements.push(json!({
            "tag": "markdown",
            "content": format!("**{}**\n{}", section_label(brief, "再看两条", "TWO MORE"), rows)
        }));
    }

    if let Some(plan) = &brief.action_plan {
        if let Some(action) = &plan.top_action {
            let experiments = plan
                .experiments
                .iter()
                .take(2)
                .map(|experiment| format!("↳ {}", experiment.title))
                .collect::<Vec<_>>()
                .join("\n");
            elements.push(json!({"tag": "hr"}));
            elements.push(json!({
                "tag": "markdown",
                "content": format!(
                    "**{} · ToT**\n### {}\n{}\n\n{}\n\n`{}`",
                    section_label(brief, "今天可以做什么", "TODAY'S MOVE"),
                    action.title,
                    action.why_now,
                    experiments,
                    action_metrics(brief, action)
                )
            }));
        }
    }

    if let Some(url) = brief_url {
        elements.push(json!({
            "tag": "button",
            "text": {"tag": "plain_text", "content": section_label(brief, "看完整简报", "Open full brief")},
            "type": "primary",
            "behaviors": [{"type": "open_url", "default_url": url}]
        }));
    }

    json!({
        "schema": "2.0",
        "config": {"wide_screen_mode": true},
        "header": {
            "template": "orange",
            "title": {"tag": "plain_text", "content": format!("Po Radar · {}", letter_label(brief))},
            "subtitle": {"tag": "plain_text", "content": format!("{} · {}", brief.date, slot_label(brief))}
        },
        "body": {"elements": elements}
    })
}

pub fn wecom_digest(brief: &Brief) -> String {
    let signals = signals(brief);
    let summary = executive_summary(brief, &signals);
    let mut out = String::new();
    out.push_str(&format!(
        "# Po Radar · {}\n{}\n\n**{}** {}\n\n{}\n\n",
        letter_label(brief),
        brief.date,
        greeting(brief),
        intro(brief, signals.len()),
        sources_line(brief)
    ));
    out.push_str(&format!(
        "**{}**\n{}\n\n",
        section_label(brief, "60 秒版", "60-SECOND BRIEF"),
        summary
    ));

    if let Some(item) = signals.first() {
        out.push_str(&format!(
            "**{} · {:.0} {}**\n[{}]({})\n{}\n\n**{}：** {}\n\n",
            section_label(brief, "今日头条", "TODAY'S LEAD"),
            item.score,
            score_unit(brief),
            title(item, brief),
            item.url,
            summary_text(item, brief),
            section_label(brief, "为什么重要", "WHY IT MATTERS"),
            why(item, brief)
        ));
    }

    if signals.len() > 1 {
        out.push_str(&format!(
            "**{}**\n",
            section_label(brief, "再看两条", "TWO MORE")
        ));
        for (index, item) in signals.iter().skip(1).take(2).enumerate() {
            out.push_str(&format!(
                "{} · [{}]({}) · {:.0}\n",
                index + 2,
                title(item, brief),
                item.url,
                item.score
            ));
        }
        out.push('\n');
    }

    if let Some(plan) = &brief.action_plan {
        if let Some(action) = &plan.top_action {
            out.push_str(&format!(
                "**{} · ToT**\n**{}**\n{}\n",
                section_label(brief, "今天可以做什么", "TODAY'S MOVE"),
                action.title,
                action.why_now
            ));
            for experiment in plan.experiments.iter().take(2) {
                out.push_str(&format!("↳ {}\n", experiment.title));
            }
            out.push('\n');
        }
    }

    if let Some(url) = public_brief_url() {
        out.push_str(&format!(
            "[{}]({})\n",
            section_label(brief, "看完整简报", "Open full brief"),
            url
        ));
    }
    truncate_bytes(out, 3900)
}

pub fn telegram_digest(brief: &Brief) -> String {
    let signals = signals(brief);
    let summary = executive_summary(brief, &signals);
    let mut out = String::new();
    out.push_str(&format!(
        "<b>Po Radar · {}</b>\n<i>{} · {}</i>\n\n<b>{}</b> {}\n\n{}\n\n",
        letter_label(brief),
        brief.date,
        slot_label(brief),
        greeting(brief),
        intro(brief, signals.len()),
        sources_line(brief)
    ));
    out.push_str(&format!(
        "<b>{}</b>\n{}\n\n",
        section_label(brief, "60 秒版", "60-SECOND BRIEF"),
        summary
    ));

    if let Some(item) = signals.first() {
        out.push_str(&format!(
            "<b>{} · {:.0} {}</b>\n<a href=\"{}\">{}</a>\n{}\n\n<b>{}：</b> {}\n\n",
            section_label(brief, "今日头条", "TODAY'S LEAD"),
            item.score,
            score_unit(brief),
            escape_html(&item.url),
            escape_html(title(item, brief)),
            escape_html(summary_text(item, brief)),
            section_label(brief, "为什么重要", "WHY IT MATTERS"),
            escape_html(why(item, brief))
        ));
    }

    if signals.len() > 1 {
        out.push_str(&format!(
            "<b>{}</b>\n",
            section_label(brief, "再看两条", "TWO MORE")
        ));
        for (index, item) in signals.iter().skip(1).take(2).enumerate() {
            out.push_str(&format!(
                "{} · <a href=\"{}\">{}</a> · {:.0}\n",
                index + 2,
                escape_html(&item.url),
                escape_html(title(item, brief)),
                item.score
            ));
        }
        out.push('\n');
    }

    if let Some(plan) = &brief.action_plan {
        if let Some(action) = &plan.top_action {
            out.push_str(&format!(
                "<b>{} · ToT</b>\n<b>{}</b>\n{}\n",
                section_label(brief, "今天可以做什么", "TODAY'S MOVE"),
                escape_html(&action.title),
                escape_html(&action.why_now)
            ));
            for experiment in plan.experiments.iter().take(2) {
                out.push_str(&format!("↳ {}\n", escape_html(&experiment.title)));
            }
            out.push('\n');
        }
    }

    if let Some(url) = public_brief_url() {
        out.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(&url),
            section_label(brief, "看完整简报", "Open full brief")
        ));
    }
    truncate_bytes(out, 3900)
}

pub fn email_digest_html(brief: &Brief) -> String {
    let signals = signals(brief);
    let summary = executive_summary_html(brief, &signals);
    let lead = signals.first();
    let quick = signals.iter().skip(1).take(2).collect::<Vec<_>>();
    let action = brief
        .action_plan
        .as_ref()
        .and_then(|plan| plan.top_action.as_ref());
    let experiments = brief
        .action_plan
        .as_ref()
        .map(|plan| {
            plan.experiments
                .iter()
                .take(2)
                .map(|experiment| {
                    format!(
                        "<div style=\"margin-top:6px;color:#596449;font-size:12px;\">↳ {}</div>",
                        escape_html(&experiment.title)
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    let lead_html = lead
        .map(|item| {
            format!(
                "<div style=\"margin:22px 0;padding:20px;border-radius:18px 18px 18px 6px;background:#f7dfce;\">
                   <div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#9f432d;text-transform:uppercase;\">{} · {:.0} {}</div>
                   <h2 style=\"margin:8px 0;font-size:22px;line-height:1.25;letter-spacing:-.03em;color:#332d27;\"><a href=\"{}\" style=\"color:#332d27;text-decoration:none;\">{}</a></h2>
                   <p style=\"margin:0;color:#675a4e;font-size:13px;line-height:1.65;\">{}</p>
                   <p style=\"margin:12px 0 0;padding-top:10px;border-top:1px solid rgba(92,65,45,.14);color:#7b4d39;font-size:12px;line-height:1.6;\"><b>{}：</b> {}</p>
                 </div>",
                section_label(brief, "今日头条", "TODAY'S LEAD"),
                item.score,
                score_unit(brief),
                escape_html(&item.url),
                escape_html(title(item, brief)),
                escape_html(summary_text(item, brief)),
                section_label(brief, "为什么重要", "WHY IT MATTERS"),
                escape_html(why(item, brief))
            )
        })
        .unwrap_or_default();
    let quick_html = if quick.is_empty() {
        String::new()
    } else {
        let rows = quick
            .iter()
            .enumerate()
            .map(|(index, item)| {
                format!(
                    "<div style=\"padding:11px 0;border-top:1px solid #eee6da;font-size:13px;line-height:1.55;\">
                       <span style=\"color:#d96849;font-family:monospace;font-size:10px;\">{:02}</span>
                       <a href=\"{}\" style=\"color:#332d27;text-decoration:none;font-weight:600;\">{}</a>
                       <span style=\"float:right;color:#c06645;font-weight:800;\">{:.0}</span>
                     </div>",
                    index + 2,
                    escape_html(&item.url),
                    escape_html(title(item, brief)),
                    item.score
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<h3 style=\"margin:22px 0 4px;font-size:14px;color:#786c5f;\">{}</h3>{}",
            section_label(brief, "再看两条", "TWO MORE"),
            rows
        )
    };
    let action_html = action
        .map(|action| {
            format!(
                "<div style=\"margin-top:24px;padding:18px;border-radius:18px;background:#e5ebdc;\">
                   <div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#72805c;text-transform:uppercase;\">{} · ToT</div>
                   <h3 style=\"margin:7px 0 5px;font-size:17px;line-height:1.35;color:#332d27;\">{}</h3>
                   <p style=\"margin:0;color:#596449;font-size:12px;line-height:1.6;\">{}</p>{}
                 </div>",
                section_label(brief, "今天可以做什么", "TODAY'S MOVE"),
                escape_html(&action.title),
                escape_html(&action.why_now),
                experiments
            )
        })
        .unwrap_or_default();
    let brief_link = public_brief_url()
        .map(|url| {
            format!(
                "<a href=\"{}\" style=\"display:inline-block;margin-top:24px;padding:10px 16px;border-radius:999px;background:#332d27;color:#fffdf8;text-decoration:none;font-size:12px;font-weight:700;\">{}</a>",
                escape_html(&url),
                section_label(brief, "看完整简报 →", "Open full brief →")
            )
        })
        .unwrap_or_default();
    let heading = match (is_zh(brief), brief.slot.as_str()) {
        (true, "evening") => "晚上好，<br>今天再看三件事。",
        (true, _) => "早上好，<br>今天先看这三件事。",
        (false, "evening") => "Good evening,<br>three things worth a second look.",
        (false, _) => "Good morning,<br>three things worth your attention.",
    };
    let intro = section_label(
        brief,
        "我用 117 个信源替你读完，留下真正值得判断和行动的部分。",
        "I read 117 sources so you can focus on what is worth judging and acting on.",
    );
    let footer = section_label(brief, "117 个信源", "117 sources");

    format!(
        "<!doctype html><html><body style=\"margin:0;padding:0;background:#f3ece2;font-family:'Avenir Next','PingFang SC','Noto Sans SC',Arial,sans-serif;color:#332d27;\">
          <table role=\"presentation\" width=\"100%\" cellspacing=\"0\" cellpadding=\"0\" style=\"background:#f3ece2;padding:28px 12px;\"><tr><td align=\"center\">
            <table role=\"presentation\" width=\"600\" cellspacing=\"0\" cellpadding=\"0\" style=\"width:100%;max-width:600px;background:#fffdf8;border:1px solid #e8ded0;border-radius:22px;overflow:hidden;\"><tr><td style=\"padding:30px 30px 26px;\">
              <div style=\"font-size:11px;letter-spacing:.12em;color:#a09383;text-transform:uppercase;\">Po Radar · {} · {}</div>
              <h1 style=\"margin:8px 0 4px;font-size:30px;line-height:1.1;letter-spacing:-.045em;color:#332d27;\">{}</h1>
              <p style=\"margin:10px 0 0;color:#786c5f;font-size:13px;line-height:1.7;\">{}</p>
              <div style=\"margin:22px 0;padding:18px 20px;border-radius:18px;background:#fbf3e6;\"><div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#a09383;\">{}</div>{}</div>
              {}{}{}{}
              <p style=\"margin:22px 0 0;color:#a09383;font-size:10px;line-height:1.6;\">Po Radar · {} · {}</p>
            </td></tr></table>
          </td></tr></table>
        </body></html>",
        escape_html(letter_label(brief)),
        escape_html(&brief.date),
        heading,
        escape_html(intro),
        section_label(brief, "60 秒版", "60-SECOND BRIEF"),
        summary,
        lead_html,
        quick_html,
        action_html,
        brief_link,
        footer,
        escape_html(&brief.date)
    )
}

fn signals(brief: &Brief) -> Vec<&TopSignal> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for item in brief.top_signals.iter() {
        if seen.insert(item.story_id.clone()) {
            out.push(item);
        }
    }
    for section in &brief.sections {
        for item in &section.items {
            if seen.insert(item.story_id.clone()) {
                out.push(item);
            }
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out
}

fn executive_summary(brief: &Brief, signals: &[&TopSignal]) -> String {
    let mut items = brief
        .executive_summary
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    if items.is_empty() {
        items = signals
            .iter()
            .take(3)
            .map(|item| {
                format!(
                    "{} · {:.0} {}",
                    title(item, brief),
                    item.score,
                    score_unit(brief)
                )
            })
            .collect();
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| format!("{}. {}", index + 1, item))
        .collect::<Vec<_>>()
        .join("\n")
}

fn executive_summary_html(brief: &Brief, signals: &[&TopSignal]) -> String {
    let mut items = brief
        .executive_summary
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    if items.is_empty() {
        items = signals
            .iter()
            .take(3)
            .map(|item| {
                format!(
                    "{} · {:.0} {}",
                    title(item, brief),
                    item.score,
                    score_unit(brief)
                )
            })
            .collect();
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "<div style=\"padding:8px 0;border-top:1px solid #e8ded0;color:#51493f;font-size:13px;line-height:1.6;\"><span style=\"color:#d96849;font-family:monospace;font-size:10px;margin-right:8px;\">{:02}</span>{}</div>",
                index + 1,
                escape_html(item)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn title<'a>(item: &'a TopSignal, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if item.title_zh.is_empty() {
            &item.title_en
        } else {
            &item.title_zh
        }
    } else if item.title_en.is_empty() {
        &item.title_zh
    } else {
        &item.title_en
    }
}

fn summary_text<'a>(item: &'a TopSignal, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if item.summary_zh.is_empty() {
            &item.summary_en
        } else {
            &item.summary_zh
        }
    } else if item.summary_en.is_empty() {
        &item.summary_zh
    } else {
        &item.summary_en
    }
}

fn why<'a>(item: &'a TopSignal, brief: &Brief) -> &'a str {
    if !item.why_it_matters.is_empty() {
        &item.why_it_matters
    } else if is_zh(brief) {
        "这条信号可能改变接下来一段时间的产品或技术判断。"
    } else {
        "This signal may change the next set of product or technology decisions."
    }
}

fn is_zh(brief: &Brief) -> bool {
    brief.lang.starts_with("zh")
}

fn slot_label(brief: &Brief) -> &'static str {
    if is_zh(brief) {
        match brief.slot.as_str() {
            "evening" => "晚报",
            _ => "早报",
        }
    } else {
        match brief.slot.as_str() {
            "evening" => "Evening",
            _ => "Morning",
        }
    }
}

fn letter_label(brief: &Brief) -> &'static str {
    if is_zh(brief) {
        match brief.slot.as_str() {
            "evening" => "晚报",
            _ => "晨读信",
        }
    } else {
        match brief.slot.as_str() {
            "evening" => "Evening Letter",
            _ => "Morning Letter",
        }
    }
}

fn greeting(brief: &Brief) -> &'static str {
    if is_zh(brief) {
        match brief.slot.as_str() {
            "evening" => "晚上好。",
            _ => "早上好。",
        }
    } else {
        match brief.slot.as_str() {
            "evening" => "Good evening.",
            _ => "Good morning.",
        }
    }
}

fn intro(brief: &Brief, count: usize) -> String {
    if is_zh(brief) {
        format!("今天有 {} 个信号值得你停下来看看。", count.max(1))
    } else {
        format!("{} signals are worth pausing for today.", count.max(1))
    }
}

fn sources_line(brief: &Brief) -> &'static str {
    section_label(
        brief,
        "用 117 个信源替你读完，留下判断和行动的空间。",
        "I read 117 sources so you can focus on judgment and action.",
    )
}

fn score_unit(brief: &Brief) -> &'static str {
    if is_zh(brief) { "分" } else { "pts" }
}

fn action_metrics(brief: &Brief, action: &ActionOption) -> String {
    if is_zh(brief) {
        format!(
            "影响 {:.0} · 可行 {:.0} · 速度 {:.0} · 置信 {:.0}%",
            action.impact,
            action.feasibility,
            action.speed,
            action.confidence * 100.0
        )
    } else {
        format!(
            "Impact {:.0} · Feasible {:.0} · Speed {:.0} · Confidence {:.0}%",
            action.impact,
            action.feasibility,
            action.speed,
            action.confidence * 100.0
        )
    }
}

fn section_label(brief: &Brief, zh: &'static str, en: &'static str) -> &'static str {
    if is_zh(brief) { zh } else { en }
}

fn public_brief_url() -> Option<String> {
    let base = std::env::var("PUBLIC_SITE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://anthonyhann.github.io/ai-news-rss".into());
    let base = base.trim_end_matches('/');
    Some(format!("{base}/#/briefs"))
}

fn truncate_bytes(value: String, limit: usize) -> String {
    if value.len() <= limit {
        return value;
    }
    let mut end = limit.saturating_sub(3);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &value[..end])
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ActionOption, ActionPlan, BriefSection, TopSignal};

    fn sample_brief() -> Brief {
        let signal = TopSignal {
            story_id: "s1".into(),
            title_zh: "OpenAI 发布新的多模态推理模型".into(),
            title_en: "OpenAI introduces a new multimodal reasoning model".into(),
            summary_zh: "新的模型支持更长上下文和工具调用。".into(),
            summary_en: "The model adds longer context and tool use.".into(),
            why_it_matters: "模型能力与工具调用正在合并。".into(),
            score: 93.0,
            url: "https://example.com/openai".into(),
            ..Default::default()
        };
        let second = TopSignal {
            story_id: "s2".into(),
            title_zh: "DeepSeek 下调 API 价格".into(),
            title_en: "DeepSeek cuts API prices".into(),
            summary_zh: "推理成本发生变化。".into(),
            summary_en: "Inference cost changes.".into(),
            why_it_matters: "值得做一次成本验证。".into(),
            score: 88.0,
            url: "https://example.com/deepseek".into(),
            ..Default::default()
        };
        let action = ActionOption {
            title: "做一次 30 分钟验证".into(),
            why_now: "先验证最小假设。".into(),
            impact: 8.0,
            feasibility: 8.0,
            speed: 7.0,
            confidence: 0.72,
            evidence: vec!["https://example.com/openai".into()],
        };
        Brief {
            lang: "zh".into(),
            date: "2026-11-21".into(),
            slot: "morning".into(),
            executive_summary: vec!["OpenAI 发布新的多模态推理模型".into()],
            sections: vec![BriefSection {
                title_zh: "模型与产品".into(),
                title_en: "Models & Products".into(),
                items: vec![signal.clone()],
            }],
            top_signals: vec![signal, second],
            action_plan: Some(ActionPlan {
                date: "2026-11-21".into(),
                candidates: vec![],
                top_action: Some(action.clone()),
                experiments: vec![action],
                watchlist: vec!["关注社区反馈".into()],
                summary: String::new(),
                model: "test".into(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn a_template_contains_all_required_sections() {
        let brief = sample_brief();
        let card = feishu_digest_card(&brief);
        let rendered = card.to_string();
        assert!(rendered.contains("60 秒版"));
        assert!(rendered.contains("今日头条"));
        assert!(rendered.contains("看完整简报"));
        assert!(!rendered.contains("\"tag\":\"action\""));
        assert!(rendered.contains("\"type\":\"open_url\""));
        assert!(wecom_digest(&brief).contains("今天可以做什么"));
        assert!(telegram_digest(&brief).contains("<b>Po Radar"));
        assert!(wecom_digest(&brief).len() <= 4096);
        assert!(telegram_digest(&brief).len() <= 4096);
        assert!(email_digest_html(&brief).contains("max-width:600px"));
    }

    #[test]
    fn truncation_is_utf8_safe() {
        let value = truncate_bytes("中文内容".repeat(1000), 20);
        assert!(value.ends_with('…'));
        assert!(value.is_char_boundary(value.len() - '…'.len_utf8()));
    }

    #[test]
    fn english_evening_template_uses_english_chrome() {
        let mut brief = sample_brief();
        brief.lang = "en".into();
        brief.slot = "evening".into();
        let card = feishu_digest_card(&brief).to_string();
        assert!(card.contains("Evening Letter"));
        assert!(card.contains("Good evening."));
        assert!(card.contains("pts"));
        assert!(wecom_digest(&brief).contains("Good evening."));
        assert!(email_digest_html(&brief).contains("Good evening"));
    }
}
