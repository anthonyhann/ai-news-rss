export type Lang = "zh" | "en";

export interface StorySource {
  item_id: string;
  source_id: string;
  source_name: string;
  kind: string;
  tier: number;
  title: string;
  url: string;
  author: string;
  published_at: string;
  excerpt?: string;
  score?: number | null;
}

export interface Story {
  id: string;
  primary_item_id: string;
  title_zh: string;
  title_en: string;
  summary_zh: string;
  summary_en: string;
  why_it_matters?: string;
  background?: string;
  community?: string;
  tags?: string[];
  score: number;
  breakdown?: Record<string, number>;
  confidence?: number;
  status?: string;
  first_seen_at: string;
  last_seen_at: string;
  sources: StorySource[];
  category?: string;
}

export interface BriefSummary {
  date: string;
  slot: string;
  lang: string;
  title: string;
  path: string;
}

export interface SourceHealth {
  source_id: string;
  name: string;
  kind: string;
  tier: number;
  enabled: boolean;
  ok: boolean;
  item_count: number;
  error?: string | null;
  checked_at: string;
}

export interface RunStats {
  id: string;
  started_at: string;
  finished_at?: string | null;
  status: string;
  fetched: number;
  stories: number;
  scored: number;
  alerts: number;
  briefs: number;
  errors?: string[];
}

export interface Brief {
  schema_version: number;
  date: string;
  slot: string;
  lang: Lang;
  executive_summary: string[];
  sections: Array<{
    title_zh: string;
    title_en: string;
    items: Array<{
      story_id: string;
      title_zh: string;
      title_en: string;
      summary_zh: string;
      summary_en: string;
      why_it_matters: string;
      background: string;
      community: string;
      tags: string[];
      score: number;
      url: string;
      sources: StorySource[];
    }>;
  }>;
  top_signals: Brief["sections"][number]["items"];
  action_plan?: {
    date: string;
    top_action?: {
      title: string;
      why_now: string;
      impact: number;
      feasibility: number;
      speed: number;
      confidence: number;
      evidence: string[];
    };
    experiments: Array<{
      title: string;
      why_now: string;
      impact: number;
      feasibility: number;
      speed: number;
      confidence: number;
      evidence: string[];
    }>;
    watchlist: string[];
    summary: string;
  };
  markdown: string;
  model: string;
}

export interface FeedPayload {
  schema_version: number;
  generated_at: string;
  total_items: number;
  items: Story[];
  sources: SourceHealth[];
  briefs: BriefSummary[];
  run: RunStats;
}
