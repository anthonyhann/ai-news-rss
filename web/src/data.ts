import type { FeedPayload } from "./types";

const demoFeed: FeedPayload = {
  schema_version: 1,
  generated_at: new Date().toISOString(),
  total_items: 3,
  sources: [],
  briefs: [],
  action_plan: {
    date: new Date().toISOString().slice(0, 10),
    candidates: [
      {
        title: "用一个最小 demo 验证今日最高分模型能力",
        why_now: "最高分信号来自官方发布，先验证再投入。",
        impact: 8,
        feasibility: 8,
        speed: 7,
        confidence: 0.72,
        evidence: ["https://openai.com"],
      },
      {
        title: "把 Agent 可靠性讨论整理成检查清单",
        why_now: "社区讨论提供了生产失败模式。",
        impact: 6,
        feasibility: 9,
        speed: 8,
        confidence: 0.7,
        evidence: ["https://news.ycombinator.com"],
      },
    ],
    top_action: {
      title: "用一个最小 demo 验证今日最高分模型能力",
      why_now: "最高分信号来自官方发布，先验证再投入。",
      impact: 8,
      feasibility: 8,
      speed: 7,
      confidence: 0.72,
      evidence: ["https://openai.com"],
    },
    experiments: [
      {
        title: "把 Agent 可靠性讨论整理成检查清单",
        why_now: "社区讨论提供了生产失败模式。",
        impact: 6,
        feasibility: 9,
        speed: 8,
        confidence: 0.7,
        evidence: ["https://news.ycombinator.com"],
      },
      {
        title: "评估 vLLM 更新对推理成本的影响",
        why_now: "推理基础设施变化会改变单位经济性。",
        impact: 7,
        feasibility: 7,
        speed: 6,
        confidence: 0.66,
        evidence: ["https://github.com"],
      },
    ],
    watchlist: ["关注模型发布后的社区评测与失败案例"],
    summary: "先验证最高分信号，再把社区讨论转成可复用判断。",
    model: "demo",
  },
  run: {
    id: "demo",
    started_at: new Date().toISOString(),
    finished_at: new Date().toISOString(),
    status: "demo",
    fetched: 3,
    stories: 3,
    scored: 3,
    alerts: 0,
    briefs: 0,
    errors: [],
  },
  items: [
    {
      id: "demo-openai",
      primary_item_id: "demo-openai",
      title_zh: "OpenAI 发布新的多模态推理模型",
      title_en: "OpenAI introduces a new multimodal reasoning model",
      summary_zh: "新的多模态推理模型支持更长上下文和工具调用，API 同步开放。",
      summary_en: "A new multimodal reasoning model adds longer context and tool use, with API access.",
      why_it_matters: "模型能力和工具调用正在合并，产品边界会继续变化。",
      score: 94,
      confidence: 0.92,
      tags: ["OpenAI", "multimodal", "reasoning"],
      first_seen_at: new Date().toISOString(),
      last_seen_at: new Date().toISOString(),
      sources: [
        {
          item_id: "demo-openai",
          source_id: "openai-blog",
          source_name: "OpenAI Blog",
          kind: "rss",
          tier: 1,
          title: "OpenAI introduces a new multimodal reasoning model",
          url: "https://openai.com",
          author: "OpenAI",
          published_at: new Date().toISOString(),
        },
      ],
    },
    {
      id: "demo-hn",
      primary_item_id: "demo-hn",
      title_zh: "HN 讨论：团队如何评估 Agent 可靠性",
      title_en: "Ask HN: How are teams evaluating agent reliability?",
      summary_zh: "开发者讨论 Agent 评估、工具失败恢复和生产日志。",
      summary_en: "Builders discuss agent evaluation, tool failure recovery and production logs.",
      why_it_matters: "可靠性正在从模型评测转向工作流评测。",
      score: 78,
      confidence: 0.8,
      tags: ["Agent", "evaluation", "production"],
      first_seen_at: new Date().toISOString(),
      last_seen_at: new Date().toISOString(),
      sources: [
        {
          item_id: "demo-hn",
          source_id: "hacker-news",
          source_name: "Hacker News",
          kind: "hackernews",
          tier: 3,
          title: "Ask HN: How are teams evaluating agent reliability?",
          url: "https://news.ycombinator.com",
          author: "hn_user",
          published_at: new Date().toISOString(),
        },
      ],
    },
    {
      id: "demo-vllm",
      primary_item_id: "demo-vllm",
      title_zh: "vLLM v0.11 发布：改进投机解码与多 LoRA 调度",
      title_en: "vLLM v0.11: speculative decoding and multi-LoRA scheduling",
      summary_zh: "新版提升推理吞吐，并补充多 LoRA 服务调度。",
      summary_en: "The release improves speculative decoding and multi-LoRA serving.",
      why_it_matters: "推理基础设施的更新会直接影响 Agent 产品的单位成本。",
      score: 86,
      confidence: 0.92,
      tags: ["vLLM", "inference", "open-source"],
      first_seen_at: new Date().toISOString(),
      last_seen_at: new Date().toISOString(),
      sources: [
        {
          item_id: "demo-vllm",
          source_id: "github-vllm",
          source_name: "vLLM Releases",
          kind: "github",
          tier: 1,
          title: "vLLM v0.11 release",
          url: "https://github.com",
          author: "vllm-project",
          published_at: new Date().toISOString(),
        },
      ],
    },
  ],
};

export async function loadFeed(): Promise<FeedPayload> {
  try {
    const response = await fetch("./data/latest.json", { cache: "no-store" });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return (await response.json()) as FeedPayload;
  } catch {
    return demoFeed;
  }
}

export async function loadBrief(path: string): Promise<unknown | null> {
  try {
    const response = await fetch(`./data/${path}`, { cache: "no-store" });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
}
