import { chromium } from "playwright";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const input = JSON.parse(fs.readFileSync(0, "utf8"));

const args = [
  "--disable-blink-features=AutomationControlled",
  "--no-sandbox",
  "--disable-dev-shm-usage",
  "--disable-gpu",
  "--disable-extensions",
  "--disable-background-networking",
  "--disable-background-timer-throttling",
  "--disable-renderer-backgrounding",
  "--disable-features=TranslateUI,BackForwardCache",
  "--mute-audio",
  "--no-first-run",
  "--no-default-browser-check",
];

function expandHome(value) {
  if (!value) return value;
  if (value.startsWith("~/")) return path.join(os.homedir(), value.slice(2));
  return value;
}

function jsonLine(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function extractScript(maxItems) {
  return ({ maxItems }) => {
    const parseCount = (node) => {
      if (!node) return null;
      const raw = (node.getAttribute("aria-label") || node.innerText || "").replace(/,/g, "");
      const m = raw.match(/([\d.]+)\s*(万|w|k|m|b|亿)?/i);
      if (!m) return null;
      let n = parseFloat(m[1]);
      const u = (m[2] || "").toLowerCase();
      if (u === "万" || u === "w") n *= 10000;
      else if (u === "亿") n *= 100000000;
      else if (u === "k") n *= 1000;
      else if (u === "m") n *= 1000000;
      else if (u === "b") n *= 1000000000;
      return Math.round(n);
    };
    const articles = [...document.querySelectorAll('article[data-testid="tweet"]')];
    return articles
      .map((a) => {
        const textEl = a.querySelector('[data-testid="tweetText"]');
        const timeEl = a.querySelector("time");
        const linkEl = [...a.querySelectorAll('a[href*="/status/"]')].find((x) => x.href);
        const authorEl = a.querySelector('a[href^="/"] [dir="ltr"]');
        return {
          url: linkEl ? linkEl.href : "",
          text: textEl ? textEl.innerText : "",
          author: authorEl ? authorEl.innerText : "",
          published_at: timeEl ? timeEl.getAttribute("datetime") : null,
          likes: parseCount(a.querySelector('[data-testid="like"]')),
          retweets: parseCount(a.querySelector('[data-testid="retweet"]')),
          replies: parseCount(a.querySelector('[data-testid="reply"]')),
          views: parseCount(a.querySelector('[aria-label*="查看"], [aria-label*="View"]')),
        };
      })
      .filter((x) => x.url && x.text)
      .slice(0, maxItems);
  };
}

function storageStateFromEnv(envName) {
  const raw = envName ? process.env[envName] : "";
  if (!raw) return null;
  try {
    return JSON.parse(Buffer.from(raw, "base64").toString("utf8"));
  } catch {
    return null;
  }
}

async function main() {
  const profileDir = expandHome(input.profile_dir);
  const storageState = storageStateFromEnv(input.storage_state_b64_env);
  const headless = input.headless !== false;
  const launchOptions = {
    headless,
    channel: process.env.PLAYWRIGHT_CHANNEL || "chrome",
    args,
  };
  const contextOptions = {
    userAgent: input.headless === false ? undefined : input.user_agent,
    locale: "zh-CN",
    viewport: { width: 1360, height: 900 },
  };
  if (input.proxy) {
    contextOptions.proxy = { server: input.proxy };
  }

  let browser;
  let context;
  if (storageState) {
    browser = await chromium.launch(launchOptions).catch(() =>
      chromium.launch({ headless, args }),
    );
    context = await browser.newContext({
      ...contextOptions,
      storageState,
    });
  } else {
    fs.mkdirSync(profileDir, { recursive: true });
    context = await chromium
      .launchPersistentContext(profileDir, { ...contextOptions, ...launchOptions })
      .catch(() =>
        chromium.launchPersistentContext(profileDir, { ...contextOptions, headless, args }),
      );
  }

  const authToken = process.env.TWITTER_AUTH_TOKEN || process.env.X_AUTH_TOKEN;
  const ct0 = process.env.TWITTER_CT0 || process.env.X_CT0;
  if (authToken && ct0) {
    await context.addCookies([
      {
        name: "auth_token",
        value: authToken,
        domain: ".x.com",
        path: "/",
        httpOnly: true,
        secure: true,
        sameSite: "Lax",
      },
      {
        name: "ct0",
        value: ct0,
        domain: ".x.com",
        path: "/",
        httpOnly: false,
        secure: true,
        sameSite: "Lax",
      },
    ]);
  }

  const page = context.pages()[0] || (await context.newPage());
  await page.goto("https://x.com/home", {
    waitUntil: "domcontentloaded",
    timeout: input.timeout_ms,
  });
  await page.waitForTimeout(5000);
  const logged = await page.evaluate(
    () => !!document.querySelector("[data-testid='SideNav_AccountSwitcher_Button']"),
  );
  if (!logged) {
    jsonLine({ ok: false, error: "X browser session is not logged in", items: [] });
    await context.close();
    if (browser) await browser.close();
    return;
  }

  await page.goto(input.list_url, {
    waitUntil: "domcontentloaded",
    timeout: input.timeout_ms,
  });
  await page.waitForTimeout(5000);
  for (let i = 0; i < input.max_scrolls; i += 1) {
    await page.mouse.wheel(0, 2600);
    await page.waitForTimeout(input.scroll_delay_ms);
  }
  const items = await page.evaluate(extractScript(input.max_items), {
    maxItems: input.max_items,
  });
  jsonLine({ ok: true, items });
  await context.close();
  if (browser) await browser.close();
}

main().catch((error) => {
  jsonLine({ ok: false, error: String(error), items: [] });
  process.exit(1);
});
