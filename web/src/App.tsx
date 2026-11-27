import { useEffect, useMemo, useState } from "react";
import { loadBrief, loadFeed } from "./data";
import PoAvatar from "./PoAvatar";
import type { Brief, BriefSummary, CreatorPack, FeedPayload, Lang, Story } from "./types";

type View = "today" | "briefs" | "sources" | "alerts" | "archive" | "about";

const nav: Array<{ id: View; zh: string; en: string; glyph: string }> = [
  { id: "today", zh: "今日雷达", en: "Today", glyph: "◉" },
  { id: "briefs", zh: "每日简报", en: "Briefs", glyph: "▤" },
  { id: "sources", zh: "信源空间", en: "Sources", glyph: "⌁" },
  { id: "alerts", zh: "高分提醒", en: "Alerts", glyph: "✦" },
  { id: "archive", zh: "时光归档", en: "Archive", glyph: "◫" },
  { id: "about", zh: "关于 Po Radar", en: "About Po Radar", glyph: "?" },
];

function routeFromHash(): View {
  const hash = window.location.hash.replace(/^#\/?/, "");
  return (nav.find((item) => item.id === hash)?.id ?? "today") as View;
}

function App() {
  const [feed, setFeed] = useState<FeedPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [view, setView] = useState<View>(routeFromHash());
  const [lang, setLang] = useState<Lang>(() => (localStorage.getItem("po-radar-lang") as Lang) || "zh");
  const [dusk, setDusk] = useState(() => localStorage.getItem("po-radar-theme") === "dusk");
  const [query, setQuery] = useState("");
  const [minScore, setMinScore] = useState(0);
  const [bookmarks, setBookmarks] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(localStorage.getItem("po-radar-bookmarks") || "[]"));
    } catch {
      return new Set();
    }
  });
  const [selectedBrief, setSelectedBrief] = useState<Brief | null>(null);
  const [briefError, setBriefError] = useState<string | null>(null);
  const [statusOpen, setStatusOpen] = useState(false);
  const [traceOpen, setTraceOpen] = useState(false);

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
    localStorage.setItem("po-radar-lang", lang);
    localStorage.setItem("po-radar-theme", dusk ? "dusk" : "oat");
    document.documentElement.dataset.theme = dusk ? "dusk" : "oat";
  }, [lang, dusk]);

  useEffect(() => {
    localStorage.setItem("po-radar-bookmarks", JSON.stringify([...bookmarks]));
  }, [bookmarks]);

  useEffect(() => {
    if (!selectedBrief || !feed || selectedBrief.lang === lang) return;
    const localized = feed.briefs.find(
      (brief) =>
        brief.date === selectedBrief.date &&
        brief.slot === selectedBrief.slot &&
        brief.lang === lang,
    );
    if (!localized) {
      setSelectedBrief(null);
      return;
    }
    let canceled = false;
    loadBrief(localized.path).then((data) => {
      if (canceled) return;
      if (data) {
        setSelectedBrief(data as Brief);
        setBriefError(null);
      } else {
        setSelectedBrief(null);
        setBriefError(
          lang === "zh"
            ? "简报正文加载失败。等待下一次静态数据同步后再试。"
            : "The brief body could not be loaded. Try again after the next static-data sync.",
        );
      }
    });
    return () => {
      canceled = true;
    };
  }, [feed, lang, selectedBrief]);

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
  const sources = feed?.sources?.length ? feed.sources : deriveSources(feed?.items ?? []);
  const t = (zh: string, en: string) => (lang === "zh" ? zh : en);

  const navigate = (next: View) => {
    window.location.hash = `#/${next}`;
    setView(next);
    setStatusOpen(false);
  };

  const openBrief = async (summary: BriefSummary) => {
    const data = await loadBrief(summary.path);
    if (data) {
      setSelectedBrief(data as Brief);
      setBriefError(null);
      navigate("briefs");
    } else {
      setSelectedBrief(null);
      setBriefError(t("简报正文加载失败，请稍后重试。", "The brief body could not be loaded. Please try again."));
      navigate("briefs");
    }
  };

  const toggleBookmark = (id: string) => {
    setBookmarks((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="app-shell">
      <aside className="spaces-rail">
        <button className="po-brand" onClick={() => navigate("today")}>
          <PoAvatar size="sm" />
          <span>
            <strong>Po Radar</strong>
            <small>Po · your companion editor</small>
          </span>
        </button>

        <button className="new-space" onClick={() => navigate("today")}>
          <span>＋</span> {t("新建空间", "New space")}
        </button>

        <div className="rail-label">{t("我的空间", "MY SPACES")}</div>
        <nav className="spaces-nav">
          {nav.map((item) => (
            <button
              key={item.id}
              className={`space-nav-item ${view === item.id ? "active" : ""}`}
              onClick={() => navigate(item.id)}
            >
              <span className="space-glyph">{item.glyph}</span>
              <span>{lang === "zh" ? item.zh : item.en}</span>
              {item.id === "alerts" && alerts.length > 0 && <em>{alerts.length}</em>}
            </button>
          ))}
        </nav>

        <div className="rail-bottom">
          <div className="rail-pulse">
            <span className="soft-dot" />
            <span>{t("每 2 小时轻轻扫描一次", "A gentle scan every 2h")}</span>
          </div>
          <button className="rail-theme" onClick={() => setDusk(!dusk)}>
            {dusk ? "☼" : "◐"} {dusk ? t("奶油柑橘", "Cream citrus") : t("暖夜", "Warm night")}
          </button>
        </div>
      </aside>

      <main className="po-main">
        <header className="po-topbar">
          {feed && (
            <StatusPill
              feed={feed}
              open={statusOpen}
              traceOpen={traceOpen}
              onToggle={() => {
                setStatusOpen(!statusOpen);
                if (statusOpen) setTraceOpen(false);
              }}
              onTrace={() => setTraceOpen(!traceOpen)}
              lang={lang}
              t={t}
            />
          )}
          <div className="top-actions">
            <button className="round-control" onClick={() => setLang(lang === "zh" ? "en" : "zh")}>
              {lang === "zh" ? "EN" : "中"}
            </button>
            <button className="round-control" onClick={() => setDusk(!dusk)} aria-label="Toggle theme">
              {dusk ? "☼" : "◐"}
            </button>
          </div>
        </header>

        {loading || !feed ? (
          <div className="po-loading">
            <PoAvatar size="lg" />
            <span>{t("Po 正在整理今天的信号…", "Po is gathering today's signals…")}</span>
          </div>
        ) : (
          <>
            {view === "today" && (
              <Today
                feed={feed}
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
                feed={feed}
                selected={selectedBrief}
                onOpen={openBrief}
                onClose={() => {
                  setSelectedBrief(null);
                  setBriefError(null);
                }}
                error={briefError}
                lang={lang}
                t={t}
              />
            )}
            {view === "sources" && <Sources sources={sources} lang={lang} t={t} />}
            {view === "alerts" && (
              <Alerts items={alerts} lang={lang} bookmarks={bookmarks} toggleBookmark={toggleBookmark} t={t} />
            )}
            {view === "archive" && <Archive feed={feed} onOpen={openBrief} lang={lang} t={t} />}
            {view === "about" && <About lang={lang} t={t} />}
          </>
        )}
      </main>
    </div>
  );
}

function StatusPill(props: {
  feed: FeedPayload;
  open: boolean;
  traceOpen: boolean;
  onToggle: () => void;
  onTrace: () => void;
  lang: Lang;
  t: (zh: string, en: string) => string;
}) {
  const { feed, open, traceOpen, onToggle, onTrace, lang, t } = props;
  const activeSources = feed.sources.filter((source) => source.item_count > 0).length || feed.sources.length;
  const state =
    feed.action_plan ? t("今天的简报已经准备好了", "Today's brief is ready")
      : feed.run.status === "success" ? t("扫描完成，等你打开简报", "Scan complete, ready when you are")
        : t("正在整理今天的信号", "Gathering today's signals");
  return (
    <div className={`status-wrap ${open ? "is-open" : ""}`}>
      <button className="status-pill" onClick={onToggle} aria-expanded={open}>
        <PoAvatar size="xs" />
        <span className="status-spark" />
        <span className="status-copy">
          <strong>{t("Po 正在阅读", "Po is reading")} {activeSources} {t("个信源", "sources")}</strong>
          <small>{state}</small>
        </span>
        <span className="status-toggle">{open ? "−" : "+"}</span>
      </button>
      {open && (
        <div className="status-tray">
          <div className="tray-head">
            <div>
              <span className="eyebrow">{t("今日清单", "TODAY / TASK LIST")}</span>
              <strong>{t("我已经替你做了这些事", "What I did for you")}</strong>
            </div>
            <span className="tray-date">{new Date(feed.generated_at).toLocaleDateString(lang === "zh" ? "zh-CN" : "en-US")}</span>
          </div>
          <div className="task-row">
            <span className="task-check">✓</span>
            <span>{t("读取信源", "Read sources")}</span>
            <b>{feed.run.fetched}</b>
          </div>
          <div className="task-row">
            <span className="task-check">✓</span>
            <span>{t("打分与去重", "Scored and deduped")}</span>
            <b>{feed.run.scored}</b>
          </div>
          <div className="task-row">
            <span className={`task-check ${feed.briefs.length ? "" : "muted"}`}>{feed.briefs.length ? "✓" : "…"}</span>
            <span>{t("准备双语简报", "Prepare bilingual briefs")}</span>
            <b>{feed.briefs.length}</b>
          </div>
          <button className="trace-toggle" onClick={onTrace}>
            {traceOpen ? "−" : "+"} {t("展开编辑脉络", "Expand editorial trace")}
          </button>
          {traceOpen && (
            <div className="trace-log">
              <p>{t("只展示可审计的编辑信息，不展示隐藏推理。", "Only auditable editorial signals are shown; hidden reasoning is not exposed.")}</p>
              <code>fetch → dedup → score → brief → action</code>
              <span>{t("模式", "Mode")} · {feed.run.status} · {feed.run.id.slice(0, 8)}</span>
            </div>
          )}
        </div>
      )}
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
  const latestBrief = feed.briefs.find((brief) => brief.lang === lang);
  const action = feed.action_plan?.top_action;
  const sourceCount = feed.sources.length;
  const pageSize = 40;
  const [visibleCount, setVisibleCount] = useState(pageSize);

  useEffect(() => {
    setVisibleCount(pageSize);
  }, [query, minScore]);

  const visibleItems = items.slice(0, visibleCount);
  const communityCount = items.filter((story) => story.sources.some((source) => ["hackernews", "reddit", "x"].includes(source.kind))).length;
  const topTags = [...new Set(items.flatMap((story) => story.tags ?? []))].slice(0, 4);
  return (
    <div className="page-stack">
      <section className="today-hero">
        <div className="hero-copy">
          <div className="hero-kicker">
            <span className="mini-pill">✳ {t("今天值得知道什么", "WHAT MATTERS TODAY")}</span>
            <span>{new Date(feed.generated_at).toLocaleDateString(lang === "zh" ? "zh-CN" : "en-US")}</span>
          </div>
          <h1>{t("早上好。先看真正重要的。", "Good morning. Start with what matters.")}</h1>
          <p className="hero-subtitle">
            {t(
              `我把 ${sourceCount} 个信源的声音收拢成一页，帮你留出判断和行动的空间。`,
              `I gathered the voices of ${sourceCount} sources into one page, leaving room for judgment and action.`,
            )}
          </p>
          {top ? (
            <div className="top-signal">
              <div className="top-signal-meta">
                <span className="score-chip">{top.score.toFixed(0)}</span>
                <span>{top.sources?.[0]?.source_name ?? "Po"}</span>
                <span className="soft-dot" />
                <span>{timeAgo(top.last_seen_at, lang)}</span>
              </div>
              <h2>{localTitle(top, lang)}</h2>
              <p>{localSummary(top, lang)}</p>
              <div className="hero-actions">
                {top.sources?.[0]?.url && (
                  <a className="primary-button" href={top.sources[0].url} target="_blank" rel="noreferrer">
                    {t("读原文", "Read source")} <span>↗</span>
                  </a>
                )}
                {latestBrief && (
                  <button className="soft-button" onClick={() => openBrief(latestBrief)}>
                    {t("打开简报", "Open brief")}
                  </button>
                )}
              </div>
            </div>
          ) : (
            <div className="empty-note">{t("今天的信号还在路上。", "Today's signals are still on their way.")}</div>
          )}
        </div>
        <div className="hero-avatar-panel">
          <PoAvatar size="xl" />
          <div className="avatar-caption">
            <span className="soft-dot" />
            {t("Po 今天也在替你读", "Po is reading so you don't have to")}
          </div>
          <div className="avatar-mood">{t("心情：暖融融地专注", "mood: warm and curious")}</div>
        </div>
      </section>

      <section className="metric-strip">
        <MetricCard label={t("今日信号", "Signals today")} value={feed.total_items} accent="peach" glyph="◉" />
        <MetricCard label={t("值得提醒", "Worth a nudge")} value={items.filter((story) => story.score >= 90).length} accent="butter" glyph="✦" />
        <MetricCard label={t("社区回声", "Community echoes")} value={communityCount} accent="sage" glyph="⌁" />
        <MetricCard label={t("最近采集", "Last collect")} value={feed.run.fetched || "—"} accent="citrus" glyph="↻" />
      </section>

      <section className="ideas-section">
        <SectionHead kicker={t("灵感表面", "IDEAS SURFACE")} title={t("今天可以带走的三个小东西", "Three small things to take with you")} />
        <div className="idea-grid">
          <div className="idea-card idea-peach">
            <span className="idea-icon">✳</span>
            <span className="eyebrow">{t("今天的线索", "THE THREAD")}</span>
            <h3>{topTags.length ? topTags.join(" · ") : t("等待更多信号", "Waiting for more signal")}</h3>
            <p>{t("从不同来源反复出现的词，往往比单条新闻更接近真实变化。", "Words that recur across sources often reveal change better than a single headline.")}</p>
          </div>
          <div className="idea-card idea-sage">
            <span className="idea-icon">↗</span>
            <span className="eyebrow">{t("一个小动作", "ONE SMALL MOVE")}</span>
            <h3>{action?.title ?? t("先打开今日简报", "Open today's brief")}</h3>
            <p>{action?.why_now ?? t("行动建议会在下一次 digest 后出现。", "The action plan appears after the next digest.")}</p>
            {latestBrief && <button className="text-button" onClick={() => openBrief(latestBrief)}>{t("看看为什么", "See why")} →</button>}
          </div>
          <div className="idea-card idea-citrus">
            <span className="idea-icon">◌</span>
            <span className="eyebrow">{t("社区温度", "COMMUNITY WEATHER")}</span>
            <h3>{communityCount} {t("条讨论正在发热", "conversations are warm")}</h3>
            <p>{t("Po 会把 HN、Reddit 和 X 的不同声部放在同一条故事旁边。", "Po places HN, Reddit and X voices beside the same story.")}</p>
            <button className="text-button" onClick={() => navigate("alerts")}>{t("查看高分提醒", "See alerts")} →</button>
          </div>
        </div>
      </section>

      <section className="content-grid">
        <div className="feed-column">
          <SectionHead kicker={t("为你留下的信号", "KEPT FOR YOU")} title={t("值得花时间的事", "Things worth your time")} />
          <div className="po-toolbar">
            <div className="po-search">
              <span>⌕</span>
              <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("搜索标题、标签或摘要…", "Search titles, tags or summaries…")} />
            </div>
            <label className="score-range">
              <span>{t("最低分", "Min score")}</span>
              <input type="range" min="0" max="100" step="5" value={minScore} onChange={(event) => setMinScore(Number(event.target.value))} />
              <b>{minScore}</b>
            </label>
          </div>
          {items.length === 0 ? (
            <div className="empty-note">{t("没有符合当前过滤条件的信号。", "No signals match the current filters.")}</div>
          ) : (
            <div className="signal-list">
              {visibleItems.map((story, index) => (
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
              {visibleCount < items.length && (
                <button className="load-more" onClick={() => setVisibleCount((count) => count + pageSize)}>
                  {t(
                    `再看 ${Math.min(pageSize, items.length - visibleCount)} 条`,
                    `Show ${Math.min(pageSize, items.length - visibleCount)} more`,
                  )}
                  <span>↓</span>
                </button>
              )}
            </div>
          )}
        </div>
        <aside className="side-stack">
          <ActionCard feed={feed} lang={lang} t={t} />
          <SourcePulse sources={feed.sources} lang={lang} t={t} />
          <div className="soft-note">
            <PoAvatar size="sm" />
            <p>{t("先看发生了什么，再看这跟你有什么关系，最后看今天能怎么用。每一条结论都能回到原文。", "See what happened, what it means for you, and what you can try today. Every claim links back to its source.")}</p>
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
  const warm = story.score >= 90 ? "hot" : story.score >= 75 ? "warm" : "cool";
  return (
    <article className="signal-card">
      <div className="signal-avatar-column">
        <span className="signal-rank">{String(rank).padStart(2, "0")}</span>
        <PoAvatar size="sm" />
      </div>
      <div className="signal-body">
        <div className="signal-topline">
          <span className={`score-chip ${warm}`}>{story.score.toFixed(0)}</span>
          <span>{story.sources?.[0]?.source_name ?? "Po source"}</span>
          <span className="soft-dot" />
          <span>{timeAgo(story.last_seen_at, lang)}</span>
          <button className={`bookmark-button ${bookmarked ? "on" : ""}`} onClick={onBookmark} aria-label="Bookmark">
            {bookmarked ? "★" : "☆"}
          </button>
        </div>
        <h3>{localTitle(story, lang)}</h3>
        <p className="signal-summary">{localSummary(story, lang)}</p>
        {localizedStoryWhy(story, lang) && (
          <div className="why-bubble">
            <span className="bubble-label">{t("这跟你有什么关系", "WHAT THIS MEANS FOR YOU")}</span>
            <p>{localizedStoryWhy(story, lang)}</p>
          </div>
        )}
        {localizedStoryTip(story, lang) && (
          <div className="why-bubble tip-bubble">
            <span className="bubble-label">{t("今天可以怎么用", "TRY IT TODAY")}</span>
            <p>{localizedStoryTip(story, lang)}</p>
          </div>
        )}
        <div className="signal-tags">
          {(story.tags ?? []).slice(0, 4).map((tag) => <span key={tag}>{tag}</span>)}
          {story.confidence != null && <span className="confidence-tag">{t("置信", "conf")} {Math.round(story.confidence * 100)}%</span>}
        </div>
        <div className="signal-links">
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
  const plan = feed.action_plan;
  const action = plan?.top_action;
  const actionTitle = action ? localizedActionText(action.title, action.title_en, lang) : "";
  const actionReady = Boolean(action) && (lang === "zh" || Boolean(actionTitle));
  const missingEnglish = lang === "en" && Boolean(action) && !actionTitle;
  return (
    <div className="side-card action-side-card">
      <div className="side-card-heading">
        <div>
          <span className="eyebrow">{t("思维树行动", "TREE OF THOUGHTS")}</span>
          <h3>{t("今天做一件小事", "One small move today")}</h3>
        </div>
        <span className="tot-badge">ToT</span>
      </div>
      {action && actionReady ? (
        <>
          <p className="action-title">{localizedActionText(action.title, action.title_en, lang)}</p>
          <p className="action-why">{localizedActionText(action.why_now, action.why_now_en, lang)}</p>
          <div className="action-metrics">
            <span>{t("影响", "impact")} {action.impact.toFixed(0)}</span>
            <span>{t("可行", "feasible")} {action.feasibility.toFixed(0)}</span>
            <span>{t("速度", "speed")} {action.speed.toFixed(0)}</span>
            <span>{t("置信", "conf")} {Math.round(action.confidence * 100)}%</span>
          </div>
          {plan?.experiments?.slice(0, 2).map((experiment) => (
            <div className="action-mini" key={experiment.title}>
              <span>↳</span>
              <span>{localizedActionText(experiment.title, experiment.title_en, lang)}</span>
            </div>
          ))}
          {localizedWatchlist(plan, lang)[0] && (
            <div className="action-watch">{t("观察", "Watch")} · {localizedWatchlist(plan, lang)[0]}</div>
          )}
        </>
      ) : (
        <>
          <p className="action-title">{missingEnglish ? "English action copy is not available yet" : t("行动建议正在路上", "The action plan is on its way")}</p>
          <p className="action-why">
            {missingEnglish
              ? "This archived plan predates the bilingual action edition. Switch to 中文 for the original action."
              : t("digest 任务会把最高分信号变成 Top 1 行动、两个实验和一个观察项。", "The digest turns top signals into one action, two experiments and a watch item.")}
          </p>
        </>
      )}
      <button className="text-button" onClick={() => window.location.hash = "#/briefs"}>
        {t("看完整行动脉络", "See the full trace")} →
      </button>
    </div>
  );
}

function SourcePulse({ sources, lang, t }: { sources: FeedPayload["sources"]; lang: Lang; t: (zh: string, en: string) => string }) {
  const top = [...sources]
    .sort((a, b) => Number(b.item_count > 0) - Number(a.item_count > 0) || a.tier - b.tier || a.name.localeCompare(b.name))
    .slice(0, 5);
  return (
    <div className="side-card source-side-card">
      <div className="side-card-heading">
        <div>
          <span className="eyebrow">{t("信源天气", "SOURCE WEATHER")}</span>
          <h3>{t("它们今天有声音", "Who spoke today")}</h3>
        </div>
        <span className="tot-badge">{sources.length}</span>
      </div>
      {top.length === 0 ? (
        <p className="muted-copy">{t("还没有信源健康数据。", "No source health data yet.")}</p>
      ) : (
        top.map((source) => (
          <div className="source-row" key={source.source_id}>
            <span className={`soft-dot ${source.ok ? "ok" : source.error ? "bad" : "idle"}`} />
            <div>
              <strong>{source.name}</strong>
              <small>{source.kind} · T{source.tier}</small>
            </div>
            <span className="source-count">{source.item_count || "—"}</span>
          </div>
        ))
      )}
    </div>
  );
}

function Briefs(props: {
  feed: FeedPayload;
  selected: Brief | null;
  onOpen: (b: BriefSummary) => void;
  onClose: () => void;
  error: string | null;
  lang: Lang;
  t: (zh: string, en: string) => string;
}) {
  const { feed, selected, onOpen, onClose, error, lang, t } = props;
  const localizedBriefs = feed.briefs.filter((brief) => brief.lang === lang);
  if (selected) {
    const briefLanguage: Lang = selected.lang === "en" ? "en" : "zh";
    const showActionPlan =
      !!selected.action_plan &&
      (briefLanguage === "zh" ||
        Boolean(selected.action_plan.top_action?.title_en) ||
        Boolean(selected.action_plan.summary_en));
    return (
      <div className="page-stack">
        <div className="brief-toolbar">
          <button className="soft-button" onClick={onClose}>← {t("返回简报", "Back to briefs")}</button>
          <span>{selected.date} · {briefSlot(selected.slot, briefLanguage)} · {selected.lang.toUpperCase()}</span>
        </div>
        <article className="brief-paper">
          <div className="brief-paper-head">
            <div>
              <span className="eyebrow">{t("Po 编辑的简报", "PO'S BRIEF")}</span>
              <h2>{selected.date}<br /><span>{briefSlot(selected.slot, briefLanguage)}</span></h2>
            </div>
            <PoAvatar size="lg" />
          </div>
          <div className="brief-summary">
            {selected.executive_summary.map((line, index) => (
              <p key={`${line}-${index}`}><span>{String(index + 1).padStart(2, "0")}</span>{localizedBriefCopy(line, briefLanguage)}</p>
            ))}
          </div>
          {selected.creator_pack && selected.creator_pack.angles?.length > 0 && (
            <CreatorBoard pack={selected.creator_pack} lang={briefLanguage} t={t} />
          )}
          {selected.sections.map((section) => (
            <section className="brief-section" key={section.title_en}>
              <h3>{briefLanguage === "zh" ? section.title_zh : section.title_en}</h3>
              {section.items.map((item) => (
                <div className="brief-item" key={item.story_id}>
                  <div className="brief-item-head">
                    <strong>{briefLanguage === "zh" ? item.title_zh : item.title_en}</strong>
                    <span className="score-chip">{item.score.toFixed(0)}</span>
                  </div>
                  <p>{briefLanguage === "zh" ? item.summary_zh : item.summary_en}</p>
                  {localizedWhy(item, briefLanguage) && (
                    <div className="why-bubble">
                      <span className="bubble-label">{t("这跟你有什么关系", "WHAT THIS MEANS FOR YOU")}</span>
                      <p>{localizedWhy(item, briefLanguage)}</p>
                    </div>
                  )}
                  {localizedBackground(item, briefLanguage) && (
                    <div className="why-bubble context-bubble">
                      <span className="bubble-label">{t("先补一句背景", "QUICK CONTEXT")}</span>
                      <p>{localizedBackground(item, briefLanguage)}</p>
                    </div>
                  )}
                  {localizedTip(item, briefLanguage) && (
                    <div className="why-bubble tip-bubble">
                      <span className="bubble-label">{t("今天可以怎么用", "TRY IT TODAY")}</span>
                      <p>{localizedTip(item, briefLanguage)}</p>
                    </div>
                  )}
                  <div className="signal-links">
                    {item.sources?.slice(0, 2).map((source) => (
                      <a key={source.item_id} href={source.url} target="_blank" rel="noreferrer">{source.source_name} ↗</a>
                    ))}
                  </div>
                </div>
              ))}
            </section>
          ))}
          {selected.action_plan && showActionPlan && (
            <section className="action-panel">
              <div className="action-panel-heading">
                <span className="eyebrow">{t("今天的行动", "TODAY'S ACTION")}</span>
                <span className="tot-badge">ToT</span>
              </div>
              <h3>
                {selected.action_plan.top_action
                  ? localizedActionText(
                      selected.action_plan.top_action.title,
                      selected.action_plan.top_action.title_en,
                      briefLanguage,
                    )
                  : localizedText(
                      selected.action_plan.summary,
                      selected.action_plan.summary_en,
                      briefLanguage,
                    )}
              </h3>
              <p>
                {selected.action_plan.top_action
                  ? localizedActionText(
                      selected.action_plan.top_action.why_now,
                      selected.action_plan.top_action.why_now_en,
                      briefLanguage,
                    )
                  : ""}
              </p>
              {!!selected.action_plan.candidates?.length && (
                <div className="action-candidates">
                  <span className="eyebrow">
                    {t("候选行动与四维评分", "CANDIDATES & SCORES")}
                  </span>
                  {selected.action_plan.candidates.map((candidate, index) => (
                    <div className="candidate-row" key={`${candidate.title}-${index}`}>
                      <span className="candidate-index">{String(index + 1).padStart(2, "0")}</span>
                      <strong>{localizedActionText(candidate.title, candidate.title_en, briefLanguage)}</strong>
                      <span className="candidate-scores">
                        {t("影响", "impact")} {candidate.impact.toFixed(0)} ·{" "}
                        {t("可行", "feasible")} {candidate.feasibility.toFixed(0)} ·{" "}
                        {t("速度", "speed")} {candidate.speed.toFixed(0)} ·{" "}
                        {t("置信", "conf")} {Math.round(candidate.confidence * 100)}%
                      </span>
                    </div>
                  ))}
                </div>
              )}
              <div className="brief-action-grid">
                <div>
                  <span className="eyebrow">{t("两个实验", "TWO EXPERIMENTS")}</span>
                  {selected.action_plan.experiments.map((experiment) => (
                    <div className="action-mini" key={experiment.title}><span>↳</span><span>{localizedActionText(experiment.title, experiment.title_en, briefLanguage)}</span></div>
                  ))}
                </div>
                <div>
                  <span className="eyebrow">{t("保持观察", "WATCHLIST")}</span>
                  {localizedWatchlist(selected.action_plan, briefLanguage).map((item) => (
                    <div className="action-mini" key={item}><span>·</span><span>{item}</span></div>
                  ))}
                </div>
              </div>
              {selected.action_plan.top_action?.evidence?.[0] && (
                <a className="story-link" href={selected.action_plan.top_action.evidence[0]} target="_blank" rel="noreferrer">
                  {t("查看行动依据", "Open evidence")} ↗
                </a>
              )}
            </section>
          )}
          {selected.action_plan && !showActionPlan && (
            <section className="action-panel">
              <p className="empty-note">
                This archived English brief predates bilingual action plans. Switch to 中文 to read the original action.
              </p>
            </section>
          )}
        </article>
      </div>
    );
  }
  return (
    <div className="page-stack">
      <SectionHead kicker={t("简报归档", "BRIEF ARCHIVE")} title={t("每个时段，一页判断", "One page of judgment per cycle")} />
      {error && <div className="empty-note" role="alert">{error}</div>}
      {localizedBriefs.length === 0 ? (
        <div className="empty-note">{t("还没有简报。digest 任务运行后会出现在这里。", "No briefs yet. They appear after the digest runs.")}</div>
      ) : (
        <div className="brief-list">
          {localizedBriefs.map((brief) => (
            <button className="brief-card" key={`${brief.date}-${brief.slot}-${brief.lang}`} onClick={() => onOpen(brief)}>
              <span className="mini-pill">{localizedSlotShort(brief.slot, lang)}</span>
              <strong>{brief.date} · {briefSlot(brief.slot, lang)}</strong>
              <p>{localizedBriefCopy(brief.title, lang)}</p>
              <span className="text-button">{t("打开", "Open")} →</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function CreatorBoard({ pack, lang, t }: { pack: CreatorPack; lang: Lang; t: (zh: string, en: string) => string }) {
  return (
    <section className="creator-board">
      <div className="creator-board-head">
        <span className="eyebrow">{t("今日创作板", "TODAY'S CREATOR BOARD")}</span>
        <h3>{localizedText(pack.daily_theme_zh, pack.daily_theme_en, lang)}</h3>
        <p>{localizedText(pack.audience_tension_zh, pack.audience_tension_en, lang)}</p>
      </div>
      <div className="creator-angle-grid">
        {pack.angles.slice(0, 3).map((angle, index) => {
          const title = localizedText(angle.title_zh, angle.title_en, lang);
          const hook = localizedText(angle.hook_zh, angle.hook_en, lang);
          const insight = localizedText(angle.angle_zh, angle.angle_en, lang);
          const format = localizedText(angle.format_zh, angle.format_en, lang);
          const evidence = localizedText(angle.evidence_zh, angle.evidence_en, lang);
          const counterpoint = localizedText(angle.counterpoint_zh, angle.counterpoint_en, lang);
          const copyText = [
            title,
            `${t("开头钩子", "Hook")}: ${hook}`,
            `${t("你的角度", "Angle")}: ${insight}`,
            `${t("适合形式", "Format")}: ${format}`,
            `${t("可引用素材", "Evidence")}: ${evidence}`,
            `${t("先别写死", "Verify first")}: ${counterpoint}`,
            angle.source_url,
          ].join("\n");
          return (
            <article className="creator-angle" key={`${angle.story_id}-${index}`}>
              <div className="creator-angle-top">
                <span className="creator-index">{String(index + 1).padStart(2, "0")}</span>
                <span className="creator-format">{format}</span>
              </div>
              <h4>{title}</h4>
              <div className="creator-field">
                <span>{t("开头钩子", "HOOK")}</span>
                <p>{hook}</p>
              </div>
              <div className="creator-field">
                <span>{t("你的角度", "YOUR ANGLE")}</span>
                <p>{insight}</p>
              </div>
              <div className="creator-field">
                <span>{t("可引用素材", "EVIDENCE")}</span>
                <p>{evidence}</p>
              </div>
              <div className="creator-field creator-counter">
                <span>{t("先别写死", "VERIFY FIRST")}</span>
                <p>{counterpoint}</p>
              </div>
              <div className="creator-actions">
                <button className="text-button" onClick={() => navigator.clipboard?.writeText(copyText)}>
                  {t("复制选题", "Copy idea")} ↗
                </button>
                {angle.source_url && (
                  <a href={angle.source_url} target="_blank" rel="noreferrer">
                    {t("看原始素材", "Open source")} ↗
                  </a>
                )}
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function Sources({ sources, lang, t }: { sources: FeedPayload["sources"]; lang: Lang; t: (zh: string, en: string) => string }) {
  return (
    <div className="page-stack">
      <SectionHead kicker={t("信源空间", "SOURCE SPACES")} title={t("哪些声音今天供血", "Which voices are feeding the radar")} />
      {sources.length === 0 ? (
        <div className="empty-note">{t("还没有信源健康数据。", "No source health data yet.")}</div>
      ) : (
        <div className="source-grid">
          {sources.map((source) => (
            <div className="source-card" key={source.source_id}>
              <div className="source-card-head">
                <span className={`soft-dot ${source.ok ? "ok" : source.error ? "bad" : "idle"}`} />
                <strong>{source.name}</strong>
                <span className="mini-pill">T{source.tier}</span>
              </div>
              <p>{source.kind} · {source.enabled ? t("启用", "enabled") : t("停用", "disabled")}</p>
              <div className="source-card-foot">
                <span>{source.item_count} {t("条", "items")}</span>
                <span>{source.ok ? t("有回应", "responding") : source.error ? t("需要看看", "needs a look") : t("未检查", "unknown")}</span>
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
    <div className="page-stack">
      <SectionHead kicker={t("高分提醒", "HIGH-SCORE ALERTS")} title={t("只留下值得打断你的事", "Only what deserves to interrupt you")} />
      {items.length === 0 ? (
        <div className="empty-note">{t("目前没有 ≥90 分的提醒。", "No alerts at or above 90 right now.")}</div>
      ) : (
        <div className="signal-list">
          {items.map((story, index) => (
            <StoryCard key={story.id} story={story} rank={index + 1} lang={lang} bookmarked={bookmarks.has(story.id)} onBookmark={() => toggleBookmark(story.id)} t={t} />
          ))}
        </div>
      )}
    </div>
  );
}

function Archive({ feed, onOpen, lang, t }: { feed: FeedPayload; onOpen: (b: BriefSummary) => void; lang: Lang; t: (zh: string, en: string) => string }) {
  const localizedBriefs = feed.briefs.filter((brief) => brief.lang === lang);
  return (
    <div className="page-stack">
      <SectionHead kicker={t("时光归档", "ARCHIVE")} title={t("把每天的判断留在可回溯的地方", "Keep each day's judgment traceable")} />
      <div className="archive-shell">
        <div className="archive-head"><span>{t("日期", "Date")}</span><span>{t("时段", "Slot")}</span><span>{t("语言", "Lang")}</span><span>{t("摘要", "Summary")}</span></div>
        {localizedBriefs.map((brief) => (
          <button className="archive-row" key={`${brief.date}-${brief.slot}-${brief.lang}`} onClick={() => onOpen(brief)}>
            <span>{brief.date}</span><span>{briefSlot(brief.slot, lang)}</span><span>{brief.lang.toUpperCase()}</span><span>{localizedBriefCopy(brief.title, lang)}</span>
          </button>
        ))}
        {localizedBriefs.length === 0 && <div className="empty-note">{t("暂无归档。", "No archive yet.")}</div>}
      </div>
    </div>
  );
}

function About({ lang, t }: { lang: Lang; t: (zh: string, en: string) => string }) {
  return (
    <div className="page-stack about-page">
      <section className="about-hero">
        <div>
          <span className="eyebrow">{t("关于 Po Radar", "ABOUT PO RADAR")}</span>
          <h1>{t("工具有可靠性，也可以有体温。", "A tool can be reliable and warm.")}</h1>
          <p>{t("Po 是 Po Radar 的陪伴式主编：柑橘橙的绒毛身体、奶油色脸和肚皮、珊瑚色耳内与脚掌，头顶还有一片叶子。界面以 Po 的视觉语言为核心：圆润原语、柔软材质、拟人化陪伴和生成式表达。它不隐藏判断，也不假装自己是人。", "Po is Po Radar's companion editor: a tangerine plush body, cream face and belly, coral inner ears and feet, and a small leaf on top. The interface is built around Po's visual language: rounded primitives, warm materials, embodied companionship and generative expression. It does not hide judgment or pretend to be human.")}</p>
        </div>
        <PoAvatar size="xl" />
      </section>
      <div className="primitive-grid">
        {[
          ["01", t("陪伴式头像", "Embodied avatar"), t("Po 不是抽象 Logo，而是一个会呼吸、会轻轻漂浮的绒毛角色。", "Po is not an abstract logo, but a plush character that breathes and gently floats.")],
          ["02", t("状态胶囊", "Status pill"), t("把扫描、评分、简报状态放在一个可展开的温和入口里。", "A gentle, expandable home for scan, score and brief status.")],
          ["03", t("灵感表面", "Ideas surface"), t("把信号转成三个可以带走的小线索。", "Turn signals into three small ideas you can take with you.")],
          ["04", t("生成式空间", "Spaces"), t("导航不是层级菜单，而是一组可停留的情报空间。", "Navigation is a set of rooms, not a deep menu tree.")],
        ].map(([number, title, description]) => (
          <div className="primitive-card" key={number}>
            <span>{number}</span>
            <h3>{title}</h3>
            <p>{description}</p>
          </div>
        ))}
      </div>
      <section className="design-language">
        <SectionHead
          kicker={t("设计语言", "DESIGN LANGUAGE")}
          title={t("软绒柑橘：从 Po 延展出的五色系统", "Soft Plush Citrus: a five-color system derived from Po")}
        />
        <div className="palette-grid">
          {[
            [t("奶油燕麦", "Oatmilk"), "#F7F1E8"],
            [t("柑橘橙", "Tangerine"), "#F19A4B"],
            [t("奶油脸", "Cream"), "#F8E8CA"],
            [t("珊瑚红", "Coral"), "#D96849"],
            [t("叶片绿", "Leaf"), "#8A9A4B"],
          ].map(([name, color]) => (
            <div className="palette-swatch" key={color}>
              <span className="swatch-color" style={{ background: color }} />
              <div>
                <strong>{name}</strong>
                <small>{color}</small>
              </div>
            </div>
          ))}
        </div>
        <div className="motion-grid">
          {[
            ["01", t("呼吸", "Breathe"), t("6s 轻微起伏", "6s, subtle scale")],
            ["02", t("漂浮", "Float"), t("6.5s 上下 4px", "6.5s, 4px drift")],
            ["03", t("状态展开", "Status reveal"), t("210ms 轻快展开", "210ms, gentle reveal")],
            ["04", t("卡片进入", "Card rise"), t("320ms 柔和不弹跳", "320ms, soft rise")],
          ].map(([number, title, description]) => (
            <div className="motion-note" key={number}>
              <span>{number}</span>
              <strong>{title}</strong>
              <small>{description}</small>
            </div>
          ))}
        </div>
      </section>
      <div className="method-grid">
        <div className="method-card">
          <span className="eyebrow">{t("编辑规则", "EDITORIAL RULES")}</span>
          <h3>{t("先确定性，后模型", "Determinism first, model second")}</h3>
          <p>{t("URL、内容哈希和 SimHash 先处理重复；模型只处理真正模糊的事件关系。", "URLs, content hashes and SimHash handle the obvious; the model handles genuinely ambiguous event relationships.")}</p>
        </div>
        <div className="method-card">
          <span className="eyebrow">{t("阅读原则", "READING RULE")}</span>
          <h3>{t("分数不是事实", "Scores are not facts")}</h3>
          <p>{t("分数是编辑排序工具。90 分以上触发提醒，60 分以上进入简报，所有结论保留原文链接。", "Scores are editorial ranking tools. 90+ triggers an alert, 60+ enters the brief, and every claim keeps its source.")}</p>
        </div>
      </div>
    </div>
  );
}

function MetricCard({ label, value, accent, glyph }: { label: string; value: number | string; accent: string; glyph: string }) {
  return (
    <div className={`metric-card ${accent}`}>
      <span className="metric-glyph">{glyph}</span>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function SectionHead({ kicker, title }: { kicker: string; title: string }) {
  return (
    <div className="section-head">
      <span className="eyebrow">{kicker}</span>
      <h2>{title}</h2>
    </div>
  );
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

function localizedText(zh: string, en: string | undefined, lang: Lang) {
  return lang === "zh" ? zh || en || "" : en || zh;
}

function localizedBriefCopy(value: string, lang: Lang) {
  if (lang === "en") {
    return value.replace(/（(\d+)\s*分）/g, " ($1 pts)");
  }
  return value.replace(/\((\d+)\s*pts\)/g, "（$1 分）");
}

function localizedActionText(zh: string, en: string | undefined, lang: Lang) {
  return lang === "zh" ? zh || en || "" : en || "";
}

function localizedStoryWhy(story: Story, lang: Lang) {
  if (lang === "zh") return humanizeLegacyWhy(story.why_it_matters || "");
  return story.why_it_matters_en || genericWhy(story.score, "en");
}

function localizedStoryTip(story: Story, lang: Lang) {
  if (lang === "zh") return story.user_tip_zh || story.user_tip_en || genericTip(story.score, "zh");
  return story.user_tip_en || genericTip(story.score, "en");
}

function localizedTip(item: Brief["sections"][number]["items"][number], lang: Lang) {
  if (lang === "zh") return item.user_tip_zh || item.user_tip_en || genericTip(item.score, "zh");
  return item.user_tip_en || genericTip(item.score, "en");
}

function localizedBackground(item: Brief["sections"][number]["items"][number], lang: Lang) {
  if (lang === "en" && /[\u3400-\u9fff]/.test(item.background || "")) return "";
  return item.background || "";
}

function localizedWatchlist(plan: FeedPayload["action_plan"] | undefined, lang: Lang) {
  if (!plan) return [];
  if (lang === "zh") return plan.watchlist;
  return plan.watchlist_en ?? [];
}

function localizedWhy(item: Brief["sections"][number]["items"][number], lang: Lang) {
  if (lang === "zh") return humanizeLegacyWhy(item.why_it_matters);
  return item.why_it_matters_en || genericWhy(item.score, "en");
}

function humanizeLegacyWhy(value: string) {
  if (value === "官方一手信息，具备明确事件与较高影响。") {
    return "这是官方发布的新变化。重点是看它有没有让你现在用的工具更快、更便宜或更好用。";
  }
  if (value === "与 AI 相关，需结合来源和社区讨论判断。") {
    return "这条消息和 AI 有关，但先别急着下结论，看看真实用户怎么用。";
  }
  return value;
}

function genericWhy(score: number, lang: Lang) {
  if (lang === "zh") {
    return score >= 90
      ? "这是今天值得注意的新变化。先看它会不会让你正在用的 AI 工具更快、更便宜或更好用。"
      : "这条消息和 AI 有关，但先看真实用户反馈，再决定要不要跟进。";
  }
  return score >= 90
    ? "This is a notable change. Check whether it makes the AI tools you use faster, cheaper or easier."
    : "This is AI-related, but wait for real user feedback before deciding whether it matters to you.";
}

function genericTip(score: number, lang: Lang) {
  if (lang === "zh") {
    return score >= 90
      ? "挑一个你每天都会做的真实任务，花 10 分钟试一次，再和现在的做法对比。"
      : "打开原文看一眼，记下它会不会影响你正在用的工具；不会就先跳过。";
  }
  return score >= 90
    ? "Spend 10 minutes trying it on one real task you do every day, then compare it with your current approach."
    : "Open the source, note whether it affects the tools you already use, and skip it if it does not.";
}

function briefSlot(slot: string, lang: Lang) {
  const evening = slot === "evening";
  if (lang === "zh") return evening ? "晚报" : "早报";
  return evening ? "Evening" : "Morning";
}

function localizedSlotShort(slot: string, lang: Lang) {
  const evening = slot === "evening";
  if (lang === "zh") return evening ? "晚" : "早";
  return evening ? "PM" : "AM";
}

function localSummary(story: Story, lang: Lang) {
  return lang === "zh" ? story.summary_zh || story.summary_en : story.summary_en || story.summary_zh;
}

function timeAgo(value: string, lang: Lang) {
  const minutes = Math.max(0, Math.round((Date.now() - new Date(value).getTime()) / 60000));
  if (minutes < 60) return lang === "zh" ? `${minutes} 分钟前` : `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return lang === "zh" ? `${hours} 小时前` : `${hours}h ago`;
  return lang === "zh" ? `${Math.round(hours / 24)} 天前` : `${Math.round(hours / 24)}d ago`;
}

export default App;
