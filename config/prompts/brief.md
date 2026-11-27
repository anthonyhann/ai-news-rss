# Plain-Language Bilingual Brief

Write a concise bilingual AI brief for ordinary AI users, not researchers or investors. Every factual claim must be grounded in the supplied sources and link to the original URL. Do not republish full article text or full transcripts.

Every item follows one structure:

1. What happened.
2. What it means for you.
3. What you can try today.

Requirements:

- Explain technical terms inline with everyday words.
- `executive_summary` has at most five bullets. Combine the event and its meaning for a normal user in one sentence. Do not start bullets with scores or benchmarks.
- Use plain section titles when evidence exists: 新模型和新工具 / New Models & Tools; 实用玩法 / Practical Uses; 行业与价格 / Industry & Pricing; 大家在聊什么 / What People Are Discussing.
- `why_it_matters` explains the practical change for a normal user, not the strategic importance for a company.
- `user_tip` is one concrete 10–30 minute action, free or low-cost, with no coding required unless clearly optional.
- `background` is one sentence explaining only the technical term or context needed to understand the item.
- Avoid unexplained terms such as agent, inference, multimodal, benchmark, fine-tuning, SOTA or MoE.
- Return a `creator_pack` for readers who are AI bloggers or creators, not just news consumers.

Creator pack:

- `daily_theme` is one cross-story editorial theme for today.
- `audience_tension` is the question readers are actually confused or curious about.
- Generate 2–3 source-grounded angles. Each angle includes:
  - `hook`: a truthful opening line, not clickbait;
  - `angle`: a distinct point of view, not a restatement of the headline;
  - `format`: short video, thread, newsletter, carousel or another practical format;
  - `evidence`: a concrete number, quote, demo or source-backed fact;
  - `counterpoint`: what the creator must verify or the strongest opposing view;
  - `source_url`: the original source.
- Prefer tension, before/after, cost changes, workflow changes, surprising demos, failures and unresolved debates.
- Do not invent quotes, numbers or links.

Use at most twelve top signals and omit sections without evidence.
