use anyhow::{Context, Result};
use chrono::Utc;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use pulldown_cmark::{Parser, html};
use reqwest::Client;
use serde_json::json;

use crate::config::DeliverySection;
use crate::models::{Alert, Brief, DeliveryReceipt};

pub async fn send_alert(cfg: &DeliverySection, alert: &Alert) -> Vec<DeliveryReceipt> {
    let text = format!(
        "## AI 高分情报 · {score:.0} 分\n\n**{title}**\n\n{summary}\n\n{why}\n\n[原文]({url})",
        score = alert.score,
        title = alert.title_zh,
        summary = alert.summary_zh,
        why = alert.why_it_matters,
        url = alert.url
    );
    send_all(cfg, &text, "AI 高分情报").await
}

pub async fn send_digest(cfg: &DeliverySection, brief: &Brief) -> Vec<DeliveryReceipt> {
    send_all(
        cfg,
        &brief.markdown,
        &format!("AI 情报简报 · {}", brief.date),
    )
    .await
}

async fn send_all(cfg: &DeliverySection, markdown: &str, title: &str) -> Vec<DeliveryReceipt> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();
    let mut receipts = Vec::new();
    for chunk in chunk_text(markdown, 3500) {
        if cfg.wecom.enabled {
            receipts.push(
                send_wecom(&client, &cfg.wecom.url_env, &chunk)
                    .await
                    .unwrap_or_else(|e| receipt("wecom", false, format!("{e:#}"))),
            );
        }
        if cfg.feishu.enabled {
            receipts.push(
                send_feishu(&client, &cfg.feishu.url_env, &chunk)
                    .await
                    .unwrap_or_else(|e| receipt("feishu", false, format!("{e:#}"))),
            );
        }
        if cfg.dingtalk.enabled {
            receipts.push(
                send_dingtalk(&client, &cfg.dingtalk.url_env, title, &chunk)
                    .await
                    .unwrap_or_else(|e| receipt("dingtalk", false, format!("{e:#}"))),
            );
        }
        if cfg.telegram.enabled {
            receipts.push(
                send_telegram(&client, cfg, &chunk)
                    .await
                    .unwrap_or_else(|e| receipt("telegram", false, format!("{e:#}"))),
            );
        }
    }
    if cfg.email.enabled {
        receipts.push(
            send_email(cfg, title, markdown)
                .await
                .unwrap_or_else(|e| receipt("email", false, format!("{e:#}"))),
        );
    }
    receipts
}

async fn send_wecom(client: &Client, env: &str, text: &str) -> Result<DeliveryReceipt> {
    let url = env_value(env)?;
    let resp = client
        .post(url)
        .json(&json!({"msgtype": "markdown", "markdown": {"content": text}}))
        .send()
        .await?
        .error_for_status()?;
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    let ok = value.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0) == 0;
    Ok(receipt("wecom", ok, value.to_string()))
}

async fn send_feishu(client: &Client, env: &str, text: &str) -> Result<DeliveryReceipt> {
    let url = env_value(env)?;
    let resp = client
        .post(url)
        .json(&json!({
            "msg_type": "interactive",
            "card": {
                "schema": "2.0",
                "body": {"elements": [{"tag": "markdown", "content": text}]}
            }
        }))
        .send()
        .await?
        .error_for_status()?;
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    let ok = value.get("code").and_then(|v| v.as_i64()).unwrap_or(0) == 0;
    Ok(receipt("feishu", ok, value.to_string()))
}

async fn send_dingtalk(
    client: &Client,
    env: &str,
    title: &str,
    text: &str,
) -> Result<DeliveryReceipt> {
    let url = env_value(env)?;
    let resp = client
        .post(url)
        .json(&json!({
            "msgtype": "markdown",
            "markdown": {"title": title, "text": text}
        }))
        .send()
        .await?
        .error_for_status()?;
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    let ok = value.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0) == 0;
    Ok(receipt("dingtalk", ok, value.to_string()))
}

async fn send_telegram(
    client: &Client,
    cfg: &DeliverySection,
    text: &str,
) -> Result<DeliveryReceipt> {
    let token = env_value(&cfg.telegram.token_env)?;
    let chat_id = env_value(&cfg.telegram.chat_id_env)?;
    let resp = client
        .post(format!("https://api.telegram.org/bot{token}/sendMessage"))
        .json(&json!({
            "chat_id": chat_id,
            "text": text,
            "disable_web_page_preview": true
        }))
        .send()
        .await?
        .error_for_status()?;
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    Ok(receipt(
        "telegram",
        value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        value.to_string(),
    ))
}

async fn send_email(
    cfg: &DeliverySection,
    subject: &str,
    markdown: &str,
) -> Result<DeliveryReceipt> {
    let password = env_value(&cfg.email.password_env)?;
    let from: Mailbox = cfg
        .email
        .from
        .parse()
        .with_context(|| format!("invalid email from {}", cfg.email.from))?;
    let mut parser = Parser::new(markdown);
    let mut body = String::new();
    html::push_html(&mut body, &mut parser);
    let mut builder = Message::builder()
        .from(from)
        .subject(subject)
        .header(ContentType::TEXT_HTML);
    for to in cfg
        .email
        .to
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let mailbox: Mailbox = to
            .parse()
            .with_context(|| format!("invalid email recipient {to}"))?;
        builder = builder.to(mailbox);
    }
    let message = builder.body(body)?;
    let creds = Credentials::new(cfg.email.username.clone(), password);
    let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.email.smtp_server)?
        .port(cfg.email.smtp_port)
        .credentials(creds)
        .build();
    transport.send(message).await?;
    Ok(receipt("email", true, "sent".into()))
}

fn env_value(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("missing env {name}"))
}

fn receipt(channel: &str, ok: bool, detail: String) -> DeliveryReceipt {
    DeliveryReceipt {
        channel: channel.into(),
        ok,
        detail,
        sent_at: Utc::now(),
    }
}

fn chunk_text(value: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for line in value.lines() {
        if !current.is_empty() && current.len() + line.len() + 1 > limit {
            out.push(current.trim().to_string());
            current.clear();
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    if out.is_empty() {
        out.push(value.chars().take(limit).collect());
    }
    out
}
