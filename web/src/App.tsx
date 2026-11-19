import { useEffect, useMemo, useState } from "react";
import { loadBrief, loadFeed } from "./data";
import type { Brief, BriefSummary, FeedPayload, Lang, Story } from "./types";

type View = "today" | "briefs" | "sources" | "alerts" | "archive" | "about";

const nav: Array<{ id: View; zh: string; en: string; icon: string }> = [
  { id: "today", zh: "今日雷达", en: "Today", icon: "◉" },
  { id: "briefs", zh: "每日简报", en: "Briefs", icon: "▤" },
  { id: "sources", zh: "信源健康", en: "Sources", icon: "⌁" },
  { id: "alerts", zh: "高分告警", en: "Alerts", icon: "△" },
  { id: "archive", zh: "归档", en: "Archive", icon: "◫" },
  { id: "about", zh: "方法", en: "About", icon: "?" },
];

function routeFromHash(): View {
  const hash = window.location.hash.replace(/^#\/?/, "");
  return (nav.find((item) => item.id === hash)?.id ?? "today") as View;
}

function App() {
  const [feed, setFeed] = useState<FeedPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [view, setView] = useState<View>(routeFromHash());
  const [lang, setLang] = useState<Lang>(() => (localStorage.getItem("ainews-lang") as Lang) || "zh");
  const [dark, setDark] = useState(() => localStorage.getItem("ainews-theme") !== "light");
  const [query, setQuery] = useState("");
  const [minScore, setMinScore] = useState(0);
  const [bookmarks, setBookmarks] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(localStorage.getItem("ainews-bookmarks") || "[]"));
    } catch {
      return new Set();
    }
  });
  const [selectedBrief, setSelectedBrief] = useState<Brief | null>(null);

  useEffect(() => {
    loadFeed().then((data) => {
      setFeed(data);
      setLoading(false);
    });
  }, []);

  useEffect(() => {
    const onHash = () => setView(routeFromHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  useEffect(() => {
    localStorage.setItem("ainews-lang", lang);
    localStorage.setItem("ainews-theme", dark ? "dark" : "light");
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  }, [lang, dark]);

  useEffect(() => {
    localStorage.setItem("ainews-bookmarks", JSON.stringify([...bookmarks]));
  }, [bookmarks]);

  const items = useMemo(() => {
    const list = feed?.items ?? [];
    const q = query.trim().toLowerCase();
    return list
      .filter((story) => {
        if (story.score < minScore) return false;
        if (!q) return true;
        const text = `${story.title_zh} ${story.title_en} ${story.summary_zh} ${story.summary_en} ${(story.tags ?? []).join(" ")}`.toLowerCase();
        return text.includes(q);
      })
      .sort((a, b) => b.score - a.score);
  }, [feed, query, minScore]);

  const top = items[0];
  const alerts = items.filter((story) => story.score >= 90);
  const sources = feed?.sources?.length
    ? feed.sources
    : deriveSources(feed?.items ?? []);

  const openBrief = async (summary: BriefSummary) => {
    const data = await loadBrief(summary.path);
    if (data) {
      setSelectedBrief(data as Brief);
      navigate("briefs");
    }
  };

  const navigate = (next: View) => {
    window.location.hash = `#/${next}`;
    setView(next);
  };

  const toggleBookmark = (id: string) => {
    setBookmarks((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const t = (zh: string, en: string) => (lang === "zh" ? zh : en);

  return (
    <div className="app-shell">
      <aside className="rail">
        <button className="brand" onClick={() => navigate("today")} aria-label="AI Radar home">
          <span className="brand-mark">AI</span>
          <span className="brand-copy">
            <strong>RADAR</strong>
            <small>intelligence / signal</small>
          </span>
        </button>
        <div className="rail-rule" />
        <nav className="nav">
          {nav.map((item) => (
            <button
              key={item.id}
              className={`nav-item ${view === item.id ? "active" : ""}`}
              onClick={() => navigate(item.id)}
            >
              <span className="nav-icon">{item.icon}</span>
              <span>{lang === "zh" ? item.zh : item.en}</span>
              {item.id === "alerts" && alerts.length > 0 && <em>{alerts.length}</em>}
            </button>
          ))}
        </nav>
        <div className="rail-foot">
          <div className="pulse">
            <span className="pulse-dot" />
            <span>{t("每 2 小时扫描", "2h scan cycle")}</span>
          </div>
          <small>{feed?.run.status ? `run ${feed.run.status}` : "local demo"}</small>
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div>
            <span className="eyebrow">{t("个人 AI 情报台", "PERSONAL AI INTELLIGENCE DESK")}</span>
            <h1>{titleFor(view, lang)}</h1>
          </div>
          <div className="top-actions">
            <button className="ghost" onClick={() => setLang(lang === "zh" ? "en" : "zh")}>
              {lang === "zh" ? "EN" : "中"}
            </button>
            <button className="ghost" onClick={() => setDark(!dark)} aria-label="Toggle theme">
              {dark ? "☼" : "◐"}
            </button>
          </div>
        </header>

        {loading ? (
          <div className="loading">Loading signals…</div>
        ) : (
          <>
            {view === "today" && (
              <Today
                feed={feed!}
                items={items}
                top={top}
                lang={lang}
                query={query}
                setQuery={setQuery}
                minScore={minScore}
                setMinScore={setMinScore}
                bookmarks={bookmarks}
                toggleBookmark={toggleBookmark}
                openBrief={openBrief}
                navigate={navigate}
                t={t}
              />
            )}
            {view === "briefs" && (
              <Briefs
                feed={feed!}
                selected={selectedBrief}
                onOpen={openBrief}
                onClose={() => setSelectedBrief(null)}
                lang={lang}
                t={t}
              />
            )}
            {view === "sources" && <Sources sources={sources} lang={lang} t={t} />}
            {view === "alerts" && (
              <Alerts items={alerts} lang={lang} bookmarks={bookmarks} toggleBookmark={toggleBookmark} t={t} />
            )}
            {view === "archive" && (
              <Archive feed={feed!} onOpen={openBrief} lang={lang} t={t} />
            )}
            {view === "about" && <About lang={lang} t={t} />}
          </>
        )}
      </main>
    </div>
  );
}

function Today(props: {
  feed: FeedPayload;
  items: Story[];
  top?: Story;
  lang: Lang;
  query: string;
  setQuery: (v: string) => void;
  minScore: number;
  setMinScore: (v: number) => void;
  bookmarks: Set<string>;
  toggleBookmark: (id: string) => void;
  openBrief: (summary: BriefSummary) => void;
  navigate: (view: View) => void;
  t: (zh: string, en: string) => string;
}) {
  const { feed, items, top, lang, query, setQuery, minScore, setMinScore, bookmarks, toggleBookmark, openBrief, navigate, t } = props;
  const latestBrief = feed.briefs.find((b) => b.lang === lang) ?? feed.briefs[0];
  return (
    <div className="stack">
      <section className="hero-grid">
        <div className="hero-card">
          <div className="hero-meta">
            <span className="tag live">LIVE SIGNAL</span>
            <span>{new Date(feed.generated_at).toLocaleString(lang === "zh" ? "zh-CN" : "en-US")}</span>
          </div>
          <h2>{top ? localTitle(top, lang) : t("等待下一次扫描", "Waiting for the next scan")}</h2>
          <p>{top ? localSummary(top, lang) : t("配置信源和模型后，这里会出现今日最高分信号。", "Configure sources and a model to see today's highest-signal story.")}</p>
          <div className="hero-actions">
            {top?.sources?.[0]?.url && (
              <a className="primary" href={top.sources[0].url} target="_blank" rel="noreferrer">
                {t("阅读原文", "Read source")} ↗
              </a>
            )}
            {latestBrief && (
              <button className="secondary" onClick={() => openBrief(latestBrief)}>
                {t("打开今日简报", "Open today's brief")}
              </button>
            )}
          </div>
          <div className="hero-score">
            <span>{top?.score.toFixed(0) ?? "—"}</span>
            <small>{t("主编评分 / 100", "editor score / 100")}</small>
          </div>
        </div>
        <div className="metric-grid">
          <Metric label={t("今日信号", "Signals today")} value={String(feed.total_items)} sub={`${items.length} ${t("达到当前过滤", "match filters")}`} />
          <Metric label={t("高分告警", "High-score alerts")} value={String(items.filter((s) => s.score >= 90).length)} sub={t("≥ 90 分", "score ≥ 90")} />
          <Metric label={t("活跃信源", "Active sources")} value={String(feed.sources.length || "—")} sub={t("含 RSS / X / 社区", "RSS / X / community")} />
          <Metric label={t("最近采集", "Last collect")} value={feed.run.fetched ? String(feed.run.fetched) : "—"} sub={feed.run.status ?? "demo"} />
        </div>
      </section>

      <section className="toolbar">
        <div className="search">
          <span>⌕</span>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("搜索标题、标签或摘要…", "Search titles, tags or summaries…")}
          />
        </div>
        <label className="range">
          <span>{t("最低分", "Min score")}</span>
          <input type="range" min="0" max="100" step="5" value={minScore} onChange={(e) => setMinScore(Number(e.target.value))} />
          <b>{minScore}</b>
        </label>
        <button className="ghost small" onClick={() => navigate("sources")}>
          {t("查看信源", "Sources")} →
        </button>
      </section>

      <section className="content-grid">
        <div className="feed-list">
          <SectionTitle kicker={t("编辑精选", "EDITOR'S PICKS")} title={t("值得花时间的信号", "Signals worth your attention")} />
          {items.length === 0 && <Empty text={t("没有符合条件的信号。", "No signals match the current filters.")} />}
          {items.map((story, index) => (
            <StoryCard
              key={story.id}
              story={story}
              rank={index + 1}
              lang={lang}
              bookmarked={bookmarks.has(story.id)}
              onBookmark={() => toggleBookmark(story.id)}
              t={t}
            />
          ))}
        </div>
        <aside className="side-stack">
          <ActionCard feed={feed} lang={lang} t={t} />
          <SourcePulse sources={feed.sources} lang={lang} t={t} />
          <div className="note-card">
            <span className="eyebrow">{t("阅读原则", "READING RULE")}</span>
            <p>{t("先看为什么重要，再看发生了什么。每条结论都应能回到原始来源。", "Read why it matters before what happened. Every claim should trace back to a source.")}</p>
          </div>
        </aside>
      </section>
    </div>
  );
}

function StoryCard(props: {
  story: Story;
  rank: number;
  lang: Lang;
  bookmarked: boolean;
  onBookmark: () => void;
  t: (zh: string, en: string) => string;
}) {
  const { story, rank, lang, bookmarked, onBookmark, t } = props;
  const scoreClass = story.score >= 90 ? "hot" : story.score >= 75 ? "warm" : "cool";
  return (
    <article className="story-card">
      <div className="story-rank">{String(rank).padStart(2, "0")}</div>
      <div className="story-main">
        <div className="story-topline">
          <span className={`score-pill ${scoreClass}`}>{story.score.toFixed(0)}</span>
          <span className="source-label">{story.sources?.[0]?.source_name ?? "unknown source"}</span>
          <span className="dot">·</span>
          <time>{timeAgo(story.last_seen_at, lang)}</time>
          <button className={`bookmark ${bookmarked ? "on" : ""}`} onClick={onBookmark} aria-label="Bookmark">
            {bookmarked ? "★" : "☆"}
          </button>
        </div>
        <h3>{localTitle(story, lang)}</h3>
        <p className="story-summary">{localSummary(story, lang)}</p>
        {story.why_it_matters && (
          <p className="why"><span>{t("为什么重要", "WHY IT MATTERS")}</span>{story.why_it_matters}</p>
        )}
        <div className="story-tags">
          {(story.tags ?? []).slice(0, 4).map((tag) => <span key={tag}>{tag}</span>)}
          {story.confidence != null && <span className="confidence">{t("置信", "conf")} {Math.round(story.confidence * 100)}%</span>}
        </div>
        <div className="story-links">
          {story.sources?.slice(0, 2).map((source) => (
            <a key={source.item_id} href={source.url} target="_blank" rel="noreferrer">
              {source.source_name} ↗
            </a>
          ))}
        </div>
      </div>
    </article>
  );
}

function ActionCard({ feed, lang, t }: { feed: FeedPayload; lang: Lang; t: (zh: string, en: string) => string }) {
  const latest = feed.briefs.find((b) => b.lang === lang) ?? feed.briefs[0];
  return (
    <div className="side-card action-card">
      <div className="side-heading">
        <span className="eyebrow">{t("行动建议", "ACTION PLAN")}</span>
        <span className="tiny">ToT</span>
      </div>
      <h3>{latest ? t("从今日信号里做一个最小验证", "Run one small experiment from today's signals") : t("等待简报生成", "Waiting for a brief")}</h3>
      <p>{latest ? t("打开今日简报，查看主编给出的 Top 1 行动、两个实验和观察项。", "Open today's brief for the editor's top action, two experiments and a watch item.") : t("先运行一次 pipeline，行动建议会出现在这里。", "Run the pipeline once and the action plan will appear here.")}</p>
      {latest && (
        <button className="link-button" onClick={() => window.location.hash = "#/briefs"}>
          {t("查看行动建议", "See the plan")} →
        </button>
      )}
    </div>
  );
}

function SourcePulse({ sources, lang, t }: { sources: FeedPayload["sources"]; lang: Lang; t: (zh: string, en: string) => string }) {
  const top = [...sources]
    .sort((a, b) => {
      const activeDelta = Number(b.item_count > 0) - Number(a.item_count > 0);
      return activeDelta || a.tier - b.tier || a.name.localeCompare(b.name);
    })
    .slice(0, 5);
  return (
    <div className="side-card">
      <div className="side-heading">
        <span className="eyebrow">{t("信源脉冲", "SOURCE PULSE")}</span>
        <span className="tiny">{sources.length || "—"}</span>
      </div>
      {top.length === 0 && <p className="muted">{t("尚无信源健康数据。运行一次采集后更新。", "No source health data yet. Run a collection to update.")}</p>}
      {top.map((source) => (
        <div className="source-row" key={source.source_id}>
          <span className={`status-dot ${source.ok ? "ok" : source.error ? "bad" : "idle"}`} />
          <div>
            <strong>{source.name}</strong>
            <small>{source.kind} · T{source.tier}</small>
          </div>
          <span className="source-count">{source.item_count || "—"}</span>
        </div>
      ))}
    </div>
  );
}

function Briefs(props: { feed: FeedPayload; selected: Brief | null; onOpen: (b: BriefSummary) => void; onClose: () => void; lang: Lang; t: (zh: string, en: string) => string }) {
  const { feed, selected, onOpen, onClose, lang, t } = props;
  if (selected) {
    return (
      <div className="stack">
        <div className="brief-toolbar">
          <button className="secondary" onClick={onClose}>← {t("返回简报列表", "Back to briefs")}</button>
          <span>{selected.date} · {selected.slot} · {selected.lang.toUpperCase()}</span>
        </div>
        <article className="brief-paper">
          <span className="eyebrow">{t("AI 情报简报", "AI INTELLIGENCE BRIEF")}</span>
          <h2>{selected.date} · {selected.slot}</h2>
          <div className="brief-summary">
            {selected.executive_summary.map((line, i) => <p key={i}><span>{String(i + 1).padStart(2, "0")}</span>{line}</p>)}
          </div>
          {selected.sections.map((section) => (
            <section key={section.title_en} className="brief-section">
              <h3>{lang === "zh" ? section.title_zh : section.title_en}</h3>
              {section.items.map((item) => (
                <div className="brief-item" key={item.story_id}>
                  <div className="brief-item-head">
                    <strong>{lang === "zh" ? item.title_zh : item.title_en}</strong>
                    <span>{item.score.toFixed(0)}</span>
                  </div>
                  <p>{lang === "zh" ? item.summary_zh : item.summary_en}</p>
                  {item.why_it_matters && <p className="why"><span>{t("为什么重要", "WHY IT MATTERS")}</span>{item.why_it_matters}</p>}
                  <div className="story-links">
                    {item.sources?.slice(0, 2).map((source) => <a key={source.item_id} href={source.url} target="_blank" rel="noreferrer">{source.source_name} ↗</a>)}
                  </div>
                </div>
              ))}
            </section>
          ))}
          {selected.action_plan && (
            <section className="action-panel">
              <span className="eyebrow">{t("今日行动", "TODAY'S ACTION")}</span>
              <h3>{selected.action_plan.top_action?.title ?? selected.action_plan.summary}</h3>
              <p>{selected.action_plan.top_action?.why_now}</p>
            </section>
          )}
        </article>
      </div>
    );
  }
  return (
    <div className="stack">
      <SectionTitle kicker={t("简报归档", "BRIEF ARCHIVE")} title={t("每个时段，一页判断", "One page of judgment per cycle")} />
      {feed.briefs.length === 0 && <Empty text={t("还没有简报。运行 `ainews run` 生成第一份。", "No briefs yet. Run `ainews run` to create the first one.")} />}
      <div className="brief-list">
        {feed.briefs.map((brief) => (
          <button className="brief-card" key={`${brief.date}-${brief.slot}-${brief.lang}`} onClick={() => onOpen(brief)}>
            <span className="tag">{brief.lang.toUpperCase()}</span>
            <strong>{brief.date} · {brief.slot}</strong>
            <p>{brief.title}</p>
            <span className="link-button">{t("打开", "Open")} →</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function Sources({ sources, lang, t }: { sources: FeedPayload["sources"]; lang: Lang; t: (zh: string, en: string) => string }) {
  return (
    <div className="stack">
      <SectionTitle kicker={t("信源健康", "SOURCE HEALTH")} title={t("哪些源在稳定供血", "Which sources are actually feeding the radar")} />
      {sources.length === 0 ? <Empty text={t("尚无健康数据。", "No health data yet.")} /> : (
        <div className="source-grid">
          {sources.map((source) => (
            <div className="source-card" key={source.source_id}>
              <div className="source-card-head">
                <span className={`status-dot ${source.ok ? "ok" : source.error ? "bad" : "idle"}`} />
                <strong>{source.name}</strong>
                <span className="tag">T{source.tier}</span>
              </div>
              <p>{source.kind} · {source.enabled ? t("启用", "enabled") : t("停用", "disabled")}</p>
              <div className="source-card-foot">
                <span>{source.item_count} {t("条", "items")}</span>
                <span>{source.ok ? t("正常", "healthy") : source.error ? t("异常", "failed") : t("未检查", "unknown")}</span>
              </div>
              {source.error && <small className="error-text">{source.error}</small>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Alerts({ items, lang, bookmarks, toggleBookmark, t }: { items: Story[]; lang: Lang; bookmarks: Set<string>; toggleBookmark: (id: string) => void; t: (zh: string, en: string) => string }) {
  return (
    <div className="stack">
      <SectionTitle kicker={t("高分告警", "HIGH-SCORE ALERTS")} title={t("只有足够重要的信号才会打断你", "Only signals important enough to interrupt you")} />
      {items.length === 0 ? <Empty text={t("当前没有 ≥90 分的告警。", "No alerts at or above 90 right now.")} /> : (
        <div className="feed-list">
          {items.map((story, index) => (
            <StoryCard key={story.id} story={story} rank={index + 1} lang={lang} bookmarked={bookmarks.has(story.id)} onBookmark={() => toggleBookmark(story.id)} t={t} />
          ))}
        </div>
      )}
    </div>
  );
}

function Archive({ feed, onOpen, lang, t }: { feed: FeedPayload; onOpen: (b: BriefSummary) => void; lang: Lang; t: (zh: string, en: string) => string }) {
  return (
    <div className="stack">
      <SectionTitle kicker={t("归档", "ARCHIVE")} title={t("把每天的判断留在可回溯的地方", "Keep each day's judgment traceable")} />
      <div className="archive-table">
        <div className="archive-head"><span>{t("日期", "Date")}</span><span>{t("时段", "Slot")}</span><span>{t("语言", "Lang")}</span><span>{t("摘要", "Summary")}</span></div>
        {feed.briefs.map((brief) => (
          <button className="archive-row" key={`${brief.date}-${brief.slot}-${brief.lang}`} onClick={() => onOpen(brief)}>
            <span>{brief.date}</span><span>{brief.slot}</span><span>{brief.lang.toUpperCase()}</span><span>{brief.title}</span>
          </button>
        ))}
        {feed.briefs.length === 0 && <Empty text={t("暂无归档。", "No archive yet.")} />}
      </div>
    </div>
  );
}

function About({ lang, t }: { lang: Lang; t: (zh: string, en: string) => string }) {
  return (
    <div className="stack about-grid">
      <div className="about-copy">
        <SectionTitle kicker={t("方法", "METHOD")} title={t("不追求更多信息，追求更少误判", "Not more information — fewer bad judgments")} />
        <p>{t("AI Radar 把多源信息先做 URL/SimHash 去重，再做一轮大模型事件合并，然后按相关性、影响力、新颖度、可信度、社区热度和可行动性打分。", "AI Radar deduplicates by URL and SimHash, then performs a model-assisted event merge, then scores relevance, impact, novelty, credibility, community signal and actionability.")}</p>
        <p>{t("分数不是事实。它是编辑排序工具：90 分以上才会触发即时提醒，60 分以上进入简报候选。每条摘要都应保留原始链接。", "Scores are editorial ranking tools, not facts. 90+ triggers an alert; 60+ enters the brief pool. Every summary should keep its source link.")}</p>
      </div>
      <div className="about-rules">
        <div><span>30</span><strong>{t("相关性", "Relevance")}</strong></div>
        <div><span>25</span><strong>{t("影响力", "Impact")}</strong></div>
        <div><span>15</span><strong>{t("新颖度", "Novelty")}</strong></div>
        <div><span>15</span><strong>{t("可信度", "Credibility")}</strong></div>
        <div><span>10</span><strong>{t("社区", "Community")}</strong></div>
        <div><span>5</span><strong>{t("可行动", "Actionability")}</strong></div>
      </div>
      <div className="about-foot">
        <span className="tag">RUST CORE</span><span className="tag">SQLITE STATE</span><span className="tag">GITHUB ACTIONS</span><span className="tag">PLAYWRIGHT / EGO-LITE</span>
      </div>
    </div>
  );
}

function Metric({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{sub}</small>
    </div>
  );
}

function SectionTitle({ kicker, title }: { kicker: string; title: string }) {
  return (
    <div className="section-title">
      <span className="eyebrow">{kicker}</span>
      <h2>{title}</h2>
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <div className="empty">{text}</div>;
}

function deriveSources(items: Story[]): FeedPayload["sources"] {
  const map = new Map<string, FeedPayload["sources"][number]>();
  for (const story of items) {
    for (const source of story.sources ?? []) {
      if (!map.has(source.source_id)) {
        map.set(source.source_id, {
          source_id: source.source_id,
          name: source.source_name,
          kind: source.kind,
          tier: source.tier,
          enabled: true,
          ok: true,
          item_count: 0,
          checked_at: story.last_seen_at,
        });
      }
      map.get(source.source_id)!.item_count += 1;
    }
  }
  return [...map.values()].sort((a, b) => a.tier - b.tier);
}

function localTitle(story: Story, lang: Lang) {
  return lang === "zh" ? story.title_zh || story.title_en : story.title_en || story.title_zh;
}

function localSummary(story: Story, lang: Lang) {
  return lang === "zh" ? story.summary_zh || story.summary_en : story.summary_en || story.summary_zh;
}

function timeAgo(value: string, lang: Lang) {
  const date = new Date(value);
  const minutes = Math.max(0, Math.round((Date.now() - date.getTime()) / 60000));
  if (minutes < 60) return lang === "zh" ? `${minutes} 分钟前` : `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return lang === "zh" ? `${hours} 小时前` : `${hours}h ago`;
  return lang === "zh" ? `${Math.round(hours / 24)} 天前` : `${Math.round(hours / 24)}d ago`;
}

function titleFor(view: View, lang: Lang) {
  const item = nav.find((entry) => entry.id === view);
  return item ? (lang === "zh" ? item.zh : item.en) : "AI Radar";
}

export default App;
