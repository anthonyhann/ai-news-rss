use std::collections::HashSet;

use serde_json::{Value, json};

use crate::models::{ActionOption, Brief, CreatorAngle, TopSignal};

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

    if let Some(pack) = &brief.creator_pack {
        let angles = pack
            .angles
            .iter()
            .take(2)
            .enumerate()
            .map(|(index, angle)| {
                format!(
                    "{}. [{}]({})\n   - {}: {}\n   - {}: {}\n   - {}: {}",
                    index + 1,
                    creator_title(angle, brief),
                    angle.source_url,
                    section_label(brief, "开头钩子", "HOOK"),
                    creator_hook(angle, brief),
                    section_label(brief, "你的角度", "ANGLE"),
                    creator_angle(angle, brief),
                    section_label(brief, "适合形式", "FORMAT"),
                    creator_format(angle, brief)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if !angles.is_empty() {
            elements.push(json!({"tag": "hr"}));
            elements.push(json!({
                "tag": "markdown",
                "content": format!(
                    "**{}**\n{}",
                    section_label(brief, "今日可以写", "WHAT TO WRITE TODAY"),
                    angles
                )
            }));
        }
    }

    if let Some(item) = lead {
        elements.push(json!({"tag": "hr"}));
        let tip_line = if user_tip(item, brief).is_empty() {
            String::new()
        } else {
            format!(
                "\n\n**{}** {}",
                section_label(brief, "今天可以怎么用", "TRY IT TODAY"),
                user_tip(item, brief)
            )
        };
        elements.push(json!({
            "tag": "markdown",
            "content": format!(
                "**{}**\n### [{}]({}) · **{:.0} {}**\n\n{}\n\n**{}** {}{}",
                section_label(brief, "今日头条", "TODAY'S LEAD"),
                title(item, brief),
                item.url,
                item.score,
                score_unit(brief),
                summary_text(item, brief),
                section_label(brief, "这跟你有什么关系", "WHAT THIS MEANS FOR YOU"),
                why(item, brief),
                tip_line
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
                .map(|experiment| format!("↳ {}", action_title(experiment, brief)))
                .collect::<Vec<_>>()
                .join("\n");
            elements.push(json!({"tag": "hr"}));
            elements.push(json!({
                "tag": "markdown",
                "content": format!(
                    "**{} · ToT**\n### {}\n{}\n\n{}\n\n`{}`",
                    section_label(brief, "今天可以做什么", "TODAY'S MOVE"),
                    action_title(action, brief),
                    action_why(action, brief),
                    experiments,
                    action_metrics(brief, action)
                )
            }));
        }
    }

    if let Some(url) = brief_url {
        let button_url = url.clone();
        let fallback_url = url.clone();
        elements.push(json!({
            "tag": "button",
            "text": {"tag": "plain_text", "content": section_label(brief, "看完整简报", "Open full brief")},
            "type": "primary",
            "behaviors": [{
                "type": "open_url",
                "default_url": button_url,
                "pc_url": url,
                "ios_url": url,
                "android_url": url
            }]
        }));
        elements.push(json!({
            "tag": "markdown",
            "content": format!(
                "{} {fallback_url}",
                section_label(
                    brief,
                    "按钮无法打开时，请复制此链接：",
                    "If the button does not open, copy this link:"
                )
            )
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
    if let Some(pack) = &brief.creator_pack {
        let angles = pack
            .angles
            .iter()
            .take(2)
            .map(|angle| {
                format!(
                    "[{}]({})\n{}：{} · {}：{}",
                    creator_title(angle, brief),
                    angle.source_url,
                    section_label(brief, "开头", "Hook"),
                    creator_hook(angle, brief),
                    section_label(brief, "形式", "Format"),
                    creator_format(angle, brief)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if !angles.is_empty() {
            out.push_str(&format!(
                "**{}**\n{}\n\n",
                section_label(brief, "今日可以写", "WHAT TO WRITE TODAY"),
                angles
            ));
        }
    }

    if let Some(item) = signals.first() {
        out.push_str(&format!(
            "**{} · {:.0} {}**\n[{}]({})\n{}\n\n**{}：** {}\n\n",
            section_label(brief, "今日头条", "TODAY'S LEAD"),
            item.score,
            score_unit(brief),
            title(item, brief),
            item.url,
            summary_text(item, brief),
            section_label(brief, "这跟你有什么关系", "WHAT THIS MEANS FOR YOU"),
            why(item, brief)
        ));
        if !user_tip(item, brief).is_empty() {
            out.push_str(&format!(
                "**{}：** {}\n\n",
                section_label(brief, "今天可以怎么用", "TRY IT TODAY"),
                user_tip(item, brief)
            ));
        }
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
                action_title(action, brief),
                action_why(action, brief)
            ));
            for experiment in plan.experiments.iter().take(2) {
                out.push_str(&format!("↳ {}\n", action_title(experiment, brief)));
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
    if let Some(pack) = &brief.creator_pack {
        let angles = pack
            .angles
            .iter()
            .take(2)
            .map(|angle| {
                format!(
                    "<a href=\"{}\">{}</a>\n{}: {} · {}: {}",
                    escape_html(&angle.source_url),
                    escape_html(creator_title(angle, brief)),
                    section_label(brief, "开头", "Hook"),
                    escape_html(creator_hook(angle, brief)),
                    section_label(brief, "形式", "Format"),
                    escape_html(creator_format(angle, brief))
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if !angles.is_empty() {
            out.push_str(&format!(
                "<b>{}</b>\n{}\n\n",
                section_label(brief, "今日可以写", "WHAT TO WRITE TODAY"),
                angles
            ));
        }
    }

    if let Some(item) = signals.first() {
        out.push_str(&format!(
            "<b>{} · {:.0} {}</b>\n<a href=\"{}\">{}</a>\n{}\n\n<b>{}：</b> {}\n\n",
            section_label(brief, "今日头条", "TODAY'S LEAD"),
            item.score,
            score_unit(brief),
            escape_html(&item.url),
            escape_html(title(item, brief)),
            escape_html(summary_text(item, brief)),
            section_label(brief, "这跟你有什么关系", "WHAT THIS MEANS FOR YOU"),
            escape_html(why(item, brief))
        ));
        if !user_tip(item, brief).is_empty() {
            out.push_str(&format!(
                "<b>{}：</b> {}\n\n",
                section_label(brief, "今天可以怎么用", "TRY IT TODAY"),
                escape_html(user_tip(item, brief))
            ));
        }
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
                escape_html(action_title(action, brief)),
                escape_html(action_why(action, brief))
            ));
            for experiment in plan.experiments.iter().take(2) {
                out.push_str(&format!(
                    "↳ {}\n",
                    escape_html(action_title(experiment, brief))
                ));
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

pub fn email_digest_html(primary: &Brief, secondary: Option<&Brief>) -> String {
    let zh_signals = signals(primary);
    let english = secondary.filter(|brief| brief.lang.starts_with("en"));
    let en_signals = english.map(signals).unwrap_or_default();
    let summary_zh = executive_summary_html(primary, &zh_signals);
    let summary_en = english
        .map(|brief| executive_summary_html(brief, &en_signals))
        .unwrap_or_else(|| {
            zh_signals
                .iter()
                .take(3)
                .enumerate()
                .map(|(index, item)| {
                    format!(
                        "<div style=\"padding:8px 0;border-top:1px solid #e8ded0;color:#51493f;font-size:13px;line-height:1.6;\"><span style=\"color:#d96849;font-family:monospace;font-size:10px;margin-right:8px;\">{:02}</span>{}</div>",
                        index + 1,
                        escape_html(&item.title_en)
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        });
    let lead = zh_signals.first().copied();
    let quick = zh_signals.iter().skip(1).take(2).collect::<Vec<_>>();
    let lead_html = lead.map(|item| {
        let english = matching_signal(&en_signals, item).unwrap_or(item);
        let tip_zh = if item.user_tip_zh.is_empty() {
            String::new()
        } else {
            format!(
                "<p style=\"margin:10px 0 0;padding:10px 12px;border-radius:12px;background:#e5ebdc;color:#596449;font-size:12px;line-height:1.6;\"><b>今天可以怎么用：</b> {}</p>",
                escape_html(&item.user_tip_zh)
            )
        };
        let tip_en = if english.user_tip_en.is_empty() {
            String::new()
        } else {
            format!(
                "<p style=\"margin:8px 0 0;padding:9px 11px;border-radius:11px;background:#edf1e7;color:#596449;font-size:11px;line-height:1.55;\"><b>Try it today:</b> {}</p>",
                escape_html(&english.user_tip_en)
            )
        };
        format!(
            "<div style=\"margin:22px 0;padding:20px;border-radius:18px 18px 18px 6px;background:#f7dfce;\">
               <div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#9f432d;text-transform:uppercase;\">今日头条 · TODAY'S LEAD · {:.0} PTS</div>
               <h2 style=\"margin:8px 0;font-size:22px;line-height:1.25;letter-spacing:-.03em;color:#332d27;\"><a href=\"{}\" style=\"color:#332d27;text-decoration:none;\">{}</a></h2>
               <p style=\"margin:0;color:#675a4e;font-size:13px;line-height:1.65;\">{}</p>
               <p style=\"margin:12px 0 0;padding-top:10px;border-top:1px solid rgba(92,65,45,.14);color:#7b4d39;font-size:12px;line-height:1.6;\"><b>这跟你有什么关系：</b> {}</p>
               {}
               <div style=\"margin-top:16px;padding-top:14px;border-top:1px dashed rgba(92,65,45,.2);\">
                 <div style=\"font-size:9px;font-weight:800;letter-spacing:.12em;color:#9f432d;text-transform:uppercase;\">ENGLISH</div>
                 <h3 style=\"margin:6px 0 5px;font-size:16px;line-height:1.35;color:#332d27;\"><a href=\"{}\" style=\"color:#332d27;text-decoration:none;\">{}</a></h3>
                 <p style=\"margin:0;color:#675a4e;font-size:12px;line-height:1.6;\">{}</p>
                 {}
               </div>
             </div>",
            item.score,
            escape_html(&item.url),
            escape_html(signal_title(item, LangSide::Zh)),
            escape_html(signal_summary(item, LangSide::Zh)),
            escape_html(why(item, primary)),
            tip_zh,
            escape_html(&english.url),
            escape_html(signal_title(english, LangSide::En)),
            escape_html(signal_summary(english, LangSide::En)),
            tip_en
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
                let english = matching_signal(&en_signals, item).unwrap_or(item);
                format!(
                    "<div style=\"padding:12px 0;border-top:1px solid #eee6da;\">
                       <div style=\"font-size:13px;line-height:1.5;\">
                         <span style=\"color:#d96849;font-family:monospace;font-size:10px;margin-right:7px;\">{:02}</span>
                         <a href=\"{}\" style=\"color:#332d27;text-decoration:none;font-weight:700;\">{}</a>
                         <span style=\"float:right;color:#c06645;font-weight:800;\">{:.0}</span>
                       </div>
                       <div style=\"margin:4px 0 0 27px;color:#8c8073;font-size:11px;line-height:1.5;\">{}</div>
                     </div>",
                    index + 2,
                    escape_html(&item.url),
                    escape_html(signal_title(item, LangSide::Zh)),
                    item.score,
                    escape_html(signal_title(english, LangSide::En))
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<h3 style=\"margin:22px 0 4px;font-size:14px;color:#786c5f;\">再看两条 · TWO MORE</h3>{}",
            rows
        )
    };
    let action_html = primary
        .action_plan
        .as_ref()
        .and_then(|plan| plan.top_action.as_ref())
        .map(|action| {
            let experiments_zh = primary
                .action_plan
                .as_ref()
                .map(|plan| {
                    plan.experiments
                        .iter()
                        .take(2)
                        .map(|experiment| {
                            format!(
                                "<div style=\"margin-top:5px;color:#596449;font-size:12px;\">↳ {}</div>",
                                escape_html(&experiment.title)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();
            let experiments_en = primary
                .action_plan
                .as_ref()
                .map(|plan| {
                    plan.experiments
                        .iter()
                        .take(2)
                        .filter(|experiment| !experiment.title_en.is_empty())
                        .map(|experiment| {
                            format!(
                                "<div style=\"margin-top:5px;color:#72805c;font-size:11px;\">↳ {}</div>",
                                escape_html(&experiment.title_en)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();
            format!(
                "<div style=\"margin-top:24px;padding:18px;border-radius:18px;background:#e5ebdc;\">
                   <div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#72805c;text-transform:uppercase;\">今天可以做什么 · TODAY'S MOVE · ToT</div>
                   <h3 style=\"margin:7px 0 5px;font-size:17px;line-height:1.35;color:#332d27;\">{}</h3>
                   <p style=\"margin:0;color:#596449;font-size:12px;line-height:1.6;\">{}</p>{}
                   <div style=\"margin-top:14px;padding-top:12px;border-top:1px dashed rgba(89,100,73,.24);\">
                     <div style=\"font-size:9px;font-weight:800;letter-spacing:.12em;color:#72805c;text-transform:uppercase;\">ENGLISH</div>
                     <h3 style=\"margin:6px 0 4px;font-size:14px;line-height:1.4;color:#332d27;\">{}</h3>
                     <p style=\"margin:0;color:#596449;font-size:11px;line-height:1.55;\">{}</p>{}
                   </div>
                 </div>",
                escape_html(&action.title),
                escape_html(&action.why_now),
                experiments_zh,
                escape_html(action_title_en(action)),
                escape_html(action_why_en(action)),
                experiments_en
            )
        })
        .unwrap_or_default();
    let (brief_link, fallback_link) = public_brief_url()
        .map(|url| {
            (
                format!(
                    "<a href=\"{}\" style=\"display:inline-block;margin-top:24px;padding:11px 18px;border-radius:999px;background:#332d27;color:#fffdf8;text-decoration:none;font-size:12px;font-weight:700;\">看完整简报 · Open full brief →</a>",
                    escape_html(&url)
                ),
                format!(
                    "<p style=\"margin:10px 0 0;color:#a09383;font-size:10px;line-height:1.6;\">按钮无法打开时，请复制：<a href=\"{}\" style=\"color:#9f432d;\">{}</a></p>",
                    escape_html(&url),
                    escape_html(&url)
                ),
            )
        })
        .unwrap_or_default();
    let creator_html = primary
        .creator_pack
        .as_ref()
        .map(|pack| {
            let angles = pack
                .angles
                .iter()
                .take(2)
                .map(|angle| {
                    format!(
                        "<div style=\"padding:13px 0;border-top:1px solid #e8ded0;\">
                           <div style=\"font-size:9px;font-weight:800;letter-spacing:.1em;color:#9f432d;text-transform:uppercase;\">{} · {}</div>
                           <h3 style=\"margin:5px 0 6px;font-size:15px;line-height:1.4;color:#332d27;\"><a href=\"{}\" style=\"color:#332d27;text-decoration:none;\">{}</a></h3>
                           <p style=\"margin:0;color:#675a4e;font-size:11px;line-height:1.6;\"><b>开头：</b>{}</p>
                           <p style=\"margin:5px 0 0;color:#675a4e;font-size:11px;line-height:1.6;\"><b>角度：</b>{}</p>
                         </div>",
                        section_label(primary, "选题", "Angle"),
                        escape_html(creator_format(angle, primary)),
                        escape_html(&angle.source_url),
                        escape_html(creator_title(angle, primary)),
                        escape_html(creator_hook(angle, primary)),
                        escape_html(creator_angle(angle, primary))
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            format!(
                "<div style=\"margin:24px 0;padding:18px 20px;border-radius:18px;background:#fff4e3;\">
                   <div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#9f432d;text-transform:uppercase;\">今日创作板 · CREATOR BOARD</div>
                   <h2 style=\"margin:7px 0 5px;font-size:19px;line-height:1.3;color:#332d27;\">{}</h2>
                   <p style=\"margin:0;color:#786c5f;font-size:11px;line-height:1.6;\">{}</p>
                   {}
                 </div>",
                escape_html(&pack.daily_theme_zh),
                escape_html(&pack.audience_tension_zh),
                angles
            )
        })
        .unwrap_or_default();
    let heading = match primary.slot.as_str() {
        "evening" => "晚上好，<br>今天再看三件事。",
        _ => "早上好，<br>今天先看这三件事。",
    };
    let heading_en = match primary.slot.as_str() {
        "evening" => "Good evening. Three things worth a second look.",
        _ => "Good morning. Three things worth your attention.",
    };
    let letter_en = match primary.slot.as_str() {
        "evening" => "EVENING LETTER",
        _ => "MORNING LETTER",
    };

    format!(
        "<!doctype html><html><body style=\"margin:0;padding:0;background:#f3ece2;font-family:'Avenir Next','PingFang SC','Noto Sans SC',Arial,sans-serif;color:#332d27;\">
          <table role=\"presentation\" width=\"100%\" cellspacing=\"0\" cellpadding=\"0\" style=\"background:#f3ece2;padding:28px 12px;\"><tr><td align=\"center\">
            <table role=\"presentation\" width=\"600\" cellspacing=\"0\" cellpadding=\"0\" style=\"width:100%;max-width:600px;background:#fffdf8;border:1px solid #e8ded0;border-radius:22px;overflow:hidden;\"><tr><td style=\"padding:30px 30px 26px;\">
              <div style=\"font-size:11px;letter-spacing:.12em;color:#a09383;text-transform:uppercase;\">Po Radar · {} / {} · {}</div>
              <h1 style=\"margin:8px 0 4px;font-size:30px;line-height:1.1;letter-spacing:-.045em;color:#332d27;\">{}</h1>
              <p style=\"margin:10px 0 0;color:#786c5f;font-size:13px;line-height:1.7;\">我用 117 个信源替你读完，只讲发生了什么、跟你有什么关系、今天能怎么用。<br><span style=\"font-size:12px;\">{} No model jargon. What happened, what it means for you, and what you can try today.</span></p>
              <div style=\"margin:22px 0;padding:18px 20px;border-radius:18px;background:#fbf3e6;\">
                <div style=\"font-size:10px;font-weight:800;letter-spacing:.1em;color:#a09383;\">60 秒版 · 60-SECOND BRIEF</div>
                {}
                <div style=\"margin:14px 0 7px;padding-top:11px;border-top:1px solid #e8ded0;font-size:10px;font-weight:800;letter-spacing:.1em;color:#a09383;\">ENGLISH</div>
                {}
              </div>
              {}{}{}{}{}{}
              <p style=\"margin:22px 0 0;color:#a09383;font-size:10px;line-height:1.6;\">Po Radar · 117 个信源 · {}</p>
            </td></tr></table>
          </td></tr></table>
        </body></html>",
        escape_html(letter_label(primary)),
        letter_en,
        escape_html(&primary.date),
        heading,
        heading_en,
        summary_zh,
        summary_en,
        creator_html,
        lead_html,
        quick_html,
        action_html,
        brief_link,
        fallback_link,
        escape_html(&primary.date)
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

#[derive(Clone, Copy)]
enum LangSide {
    Zh,
    En,
}

fn matching_signal<'a>(pool: &[&'a TopSignal], item: &TopSignal) -> Option<&'a TopSignal> {
    pool.iter()
        .find(|candidate| candidate.story_id == item.story_id)
        .copied()
}

fn signal_title(item: &TopSignal, side: LangSide) -> &str {
    match side {
        LangSide::Zh if item.title_zh.is_empty() => &item.title_en,
        LangSide::Zh => &item.title_zh,
        LangSide::En if item.title_en.is_empty() => &item.title_zh,
        LangSide::En => &item.title_en,
    }
}

fn signal_summary(item: &TopSignal, side: LangSide) -> &str {
    match side {
        LangSide::Zh if item.summary_zh.is_empty() => &item.summary_en,
        LangSide::Zh => &item.summary_zh,
        LangSide::En if item.summary_en.is_empty() => &item.summary_zh,
        LangSide::En => &item.summary_en,
    }
}

fn action_title<'a>(action: &'a ActionOption, brief: &Brief) -> &'a str {
    if !is_zh(brief) && !action.title_en.is_empty() {
        &action.title_en
    } else {
        &action.title
    }
}

fn action_why<'a>(action: &'a ActionOption, brief: &Brief) -> &'a str {
    if !is_zh(brief) && !action.why_now_en.is_empty() {
        &action.why_now_en
    } else {
        &action.why_now
    }
}

fn action_title_en(action: &ActionOption) -> &str {
    if action.title_en.is_empty() {
        &action.title
    } else {
        &action.title_en
    }
}

fn action_why_en(action: &ActionOption) -> &str {
    if action.why_now_en.is_empty() {
        &action.why_now
    } else {
        &action.why_now_en
    }
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
    if is_zh(brief) {
        if !item.why_it_matters.is_empty() {
            &item.why_it_matters
        } else {
            "这条信号可能改变接下来一段时间的产品或技术判断。"
        }
    } else if !item.why_it_matters_en.is_empty() {
        &item.why_it_matters_en
    } else {
        "This signal may change the next set of product or technology decisions."
    }
}

fn user_tip<'a>(item: &'a TopSignal, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if !item.user_tip_zh.is_empty() {
            &item.user_tip_zh
        } else {
            &item.user_tip_en
        }
    } else {
        &item.user_tip_en
    }
}

fn creator_title<'a>(angle: &'a CreatorAngle, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if angle.title_zh.is_empty() {
            &angle.title_en
        } else {
            &angle.title_zh
        }
    } else if angle.title_en.is_empty() {
        &angle.title_zh
    } else {
        &angle.title_en
    }
}

fn creator_hook<'a>(angle: &'a CreatorAngle, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if angle.hook_zh.is_empty() {
            &angle.hook_en
        } else {
            &angle.hook_zh
        }
    } else if angle.hook_en.is_empty() {
        &angle.hook_zh
    } else {
        &angle.hook_en
    }
}

fn creator_angle<'a>(angle: &'a CreatorAngle, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if angle.angle_zh.is_empty() {
            &angle.angle_en
        } else {
            &angle.angle_zh
        }
    } else if angle.angle_en.is_empty() {
        &angle.angle_zh
    } else {
        &angle.angle_en
    }
}

fn creator_format<'a>(angle: &'a CreatorAngle, brief: &Brief) -> &'a str {
    if is_zh(brief) {
        if angle.format_zh.is_empty() {
            &angle.format_en
        } else {
            &angle.format_zh
        }
    } else if angle.format_en.is_empty() {
        &angle.format_zh
    } else {
        &angle.format_en
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
        "用 117 个信源替你读完，只讲人话：发生了什么、跟你有什么关系、今天能怎么用。",
        "I read 117 sources and keep it plain: what happened, what it means for you, and what you can try today.",
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
    use crate::models::{
        ActionOption, ActionPlan, BriefSection, CreatorAngle, CreatorPack, TopSignal,
    };

    fn sample_brief() -> Brief {
        let signal = TopSignal {
            story_id: "s1".into(),
            title_zh: "OpenAI 发布新的多模态推理模型".into(),
            title_en: "OpenAI introduces a new multimodal reasoning model".into(),
            summary_zh: "新的模型支持更长上下文和工具调用。".into(),
            summary_en: "The model adds longer context and tool use.".into(),
            why_it_matters: "模型能力与工具调用正在合并。".into(),
            why_it_matters_en: "Model capabilities and tool use are converging.".into(),
            user_tip_zh: "拿一张工作截图，让新模型整理成三条待办，看看是否比现在快。".into(),
            user_tip_en:
                "Give the new model a work screenshot and ask for three action items, then compare the speed."
                    .into(),
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
            user_tip_zh: "用你最近一次 API 账单估算，看看同样的任务能省多少钱。".into(),
            user_tip_en:
                "Use your latest API bill to estimate how much the same task would cost now.".into(),
            score: 88.0,
            url: "https://example.com/deepseek".into(),
            ..Default::default()
        };
        let action = ActionOption {
            title: "做一次 30 分钟验证".into(),
            title_en: "Run a 30-minute validation".into(),
            why_now: "先验证最小假设。".into(),
            why_now_en: "Validate the smallest assumption first.".into(),
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
                watchlist_en: vec!["Watch community feedback".into()],
                summary: String::new(),
                summary_en: String::new(),
                model: "test".into(),
            }),
            creator_pack: Some(CreatorPack {
                daily_theme_zh: "AI 工具正在从回答问题走向替你完成工作。".into(),
                daily_theme_en: "AI tools are moving from answering to doing the work.".into(),
                audience_tension_zh: "读者想知道这次更新到底能不能省时间。".into(),
                audience_tension_en: "Readers want to know whether this saves real time.".into(),
                angles: vec![CreatorAngle {
                    story_id: "s1".into(),
                    title_zh: "OpenAI 发布新的多模态推理模型".into(),
                    title_en: "OpenAI introduces a new multimodal reasoning model".into(),
                    hook_zh: "别急着换工具：先看它到底改变了哪一步。".into(),
                    hook_en: "Before switching tools, see what actually changed.".into(),
                    angle_zh: "从普通人一天的文档工作出发，而不是从参数出发。".into(),
                    angle_en: "Start with one real document task, not the model specs.".into(),
                    format_zh: "30 秒短视频 + 3 张图文卡".into(),
                    format_en: "30-second short video + 3-slide carousel".into(),
                    evidence_zh: "官方发布和原文链接。".into(),
                    evidence_en: "Official announcement and source link.".into(),
                    counterpoint_zh: "先用真实任务验证，不要只看演示。".into(),
                    counterpoint_en: "Test a real task before trusting the demo.".into(),
                    source_url: "https://example.com/openai".into(),
                }],
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
        assert!(rendered.contains("这跟你有什么关系"));
        assert!(rendered.contains("今天可以怎么用"));
        assert!(rendered.contains("今日可以写"));
        assert!(!rendered.contains("\"tag\":\"action\""));
        assert!(rendered.contains("\"type\":\"open_url\""));
        assert!(rendered.contains("\"pc_url\""));
        assert!(rendered.contains("\"ios_url\""));
        assert!(rendered.contains("\"android_url\""));
        assert!(wecom_digest(&brief).contains("今天可以做什么"));
        assert!(telegram_digest(&brief).contains("<b>Po Radar"));
        assert!(wecom_digest(&brief).len() <= 4096);
        assert!(telegram_digest(&brief).len() <= 4096);
        let email = email_digest_html(&brief, None);
        assert!(email.contains("max-width:600px"));
        assert!(email.contains("OpenAI 发布新的多模态推理模型"));
        assert!(email.contains("OpenAI introduces a new multimodal reasoning model"));
        assert!(email.contains("拿一张工作截图"));
        assert!(email.contains("CREATOR BOARD"));
        assert!(email.contains(&public_brief_url().unwrap()));
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
        assert!(card.contains("Model capabilities and tool use are converging."));
        assert!(card.contains("TRY IT TODAY"));
        assert!(card.contains("WHAT TO WRITE TODAY"));
        assert!(!card.contains("模型能力与工具调用正在合并。"));
        assert!(wecom_digest(&brief).contains("Good evening."));
        let mut zh = sample_brief();
        zh.slot = "evening".into();
        let email = email_digest_html(&zh, Some(&brief));
        assert!(email.contains("Good evening"));
        assert!(email.contains("晚上好"));
    }

    #[test]
    #[ignore]
    fn write_bilingual_email_preview() {
        let path = std::env::var("PO_EMAIL_PREVIEW")
            .unwrap_or_else(|_| "/tmp/po-email-preview.html".into());
        let zh = sample_brief();
        let mut en = zh.clone();
        en.lang = "en".into();
        en.executive_summary = vec![
            "OpenAI introduces a new multimodal reasoning model (93 pts)".into(),
            "DeepSeek cuts API prices (88 pts)".into(),
        ];
        std::fs::write(path, email_digest_html(&zh, Some(&en))).unwrap();
    }
}
