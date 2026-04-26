#!/usr/bin/env node

const DEFAULT_MAX_RESULTS = 8;

function parseArgs(argv) {
  const [action = "search", ...rest] = argv;
  const opts = {
    action,
    maxResults: DEFAULT_MAX_RESULTS,
    positional: [],
  };

  for (let i = 0; i < rest.length; i += 1) {
    const value = rest[i];
    if (value === "--max-results" && rest[i + 1]) {
      opts.maxResults = Number.parseInt(rest[i + 1], 10) || DEFAULT_MAX_RESULTS;
      i += 1;
      continue;
    }
    opts.positional.push(value);
  }

  return opts;
}

function decodeHtml(text) {
  return text
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/\s+/g, " ")
    .trim();
}

function stripHtml(text) {
  return decodeHtml(
    text
      .replace(/<script[\s\S]*?<\/script>/gi, " ")
      .replace(/<style[\s\S]*?<\/style>/gi, " ")
      .replace(/<[^>]+>/g, " ")
  );
}

function uniqueByUrl(results) {
  const seen = new Set();
  return results.filter((item) => {
    if (!item.url || seen.has(item.url)) return false;
    seen.add(item.url);
    return true;
  });
}

function extractDuckDuckGoResults(html, maxResults) {
  const results = [];
  const regex = /<a[^>]+class="result__a"[^>]+href="([^"]+)"[^>]*>([\s\S]*?)<\/a>[\s\S]*?<a[^>]+class="result__snippet"[^>]*>([\s\S]*?)<\/a>/gi;
  let match;
  while ((match = regex.exec(html)) && results.length < maxResults) {
    let url = decodeHtml(match[1]);
    try {
      const parsed = new URL(url);
      const uddg = parsed.searchParams.get("uddg");
      if (uddg) url = uddg;
    } catch {
      // Keep original URL if parsing fails.
    }
    results.push({
      title: stripHtml(match[2]),
      url,
      snippet: stripHtml(match[3]),
    });
  }
  return uniqueByUrl(results).slice(0, maxResults);
}

function extractBingResults(html, maxResults) {
  const results = [];
  const regex = /<li class="b_algo"[\s\S]*?<h2[^>]*>[\s\S]*?<a[^>]+href="([^"]+)"[^>]*>([\s\S]*?)<\/a>[\s\S]*?(?:<p[^>]*>([\s\S]*?)<\/p>)?/gi;
  let match;
  while ((match = regex.exec(html)) && results.length < maxResults) {
    results.push({
      title: stripHtml(match[2] || ""),
      url: decodeHtml(match[1] || ""),
      snippet: stripHtml(match[3] || ""),
    });
  }
  return uniqueByUrl(results).slice(0, maxResults);
}

async function requestText(url) {
  const response = await fetch(url, {
    headers: {
      "user-agent":
        "Mozilla/5.0 (Macintosh; Intel Mac OS X) AppleWebKit/537.36 (KHTML, like Gecko) Chrome Safari",
      accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    },
  });
  return response.text();
}

async function httpSearch(query, maxResults) {
  const warnings = [];

  try {
    const html = await requestText(`https://duckduckgo.com/html/?q=${encodeURIComponent(query)}`);
    const results = extractDuckDuckGoResults(html, maxResults);
    if (results.length > 0) {
      return { driver: "http_fallback", provider: "duckduckgo", query, results, warnings };
    }
  } catch (error) {
    warnings.push(`DuckDuckGo failed: ${error.message}`);
  }

  try {
    const html = await requestText(`https://www.bing.com/search?q=${encodeURIComponent(query)}`);
    const results = extractBingResults(html, maxResults);
    if (results.length > 0) {
      return { driver: "http_fallback", provider: "bing", query, results, warnings };
    }
  } catch (error) {
    warnings.push(`Bing failed: ${error.message}`);
  }

  return { driver: "http_fallback", provider: "none", query, results: [], warnings };
}

async function playwrightSearch(query, maxResults) {
  const { chromium } = await import("playwright");
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.goto(`https://duckduckgo.com/?q=${encodeURIComponent(query)}`, {
      waitUntil: "domcontentloaded",
      timeout: 25000,
    });
    await page.waitForTimeout(1200);
    const results = await page.$$eval("a", (anchors) =>
      anchors
        .map((a) => ({
          title: (a.textContent || "").trim(),
          url: a.href,
          snippet: "",
        }))
        .filter((item) => item.title && item.url && !item.url.includes("duckduckgo.com"))
    );
    return uniqueByUrl(results).slice(0, maxResults);
  } finally {
    await browser.close();
  }
}

async function search(query, maxResults) {
  try {
    const results = await playwrightSearch(query, maxResults);
    if (results.length > 0) {
      return { driver: "playwright", query, results };
    }
  } catch (error) {
    return {
      warning: `Playwright unavailable or failed: ${error.message}`,
      ...(await httpSearch(query, maxResults)),
    };
  }

  return httpSearch(query, maxResults);
}

async function fetchUrl(url) {
  const response = await fetch(url, {
    headers: {
      "user-agent":
        "Mozilla/5.0 (Macintosh; Intel Mac OS X) AppleWebKit/537.36 (KHTML, like Gecko) Chrome Safari",
    },
  });
  const html = await response.text();
  const title = stripHtml((html.match(/<title[^>]*>([\s\S]*?)<\/title>/i) || [null, ""])[1]);
  const descriptionMatch =
    html.match(/<meta[^>]+name=["']description["'][^>]+content=["']([^"']+)["']/i) ||
    html.match(/<meta[^>]+content=["']([^"']+)["'][^>]+name=["']description["']/i);
  const description = descriptionMatch ? decodeHtml(descriptionMatch[1]) : "";
  const text = stripHtml(html).slice(0, 12000);

  return {
    driver: "http_fetch",
    url,
    status: response.status,
    title,
    description,
    text,
  };
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));

  if (opts.action === "search") {
    const query = opts.positional.join(" ").trim();
    if (!query) throw new Error("search requires a query in args");
    console.log(JSON.stringify(await search(query, opts.maxResults), null, 2));
    return;
  }

  if (opts.action === "fetch_url") {
    const [url] = opts.positional;
    if (!url) throw new Error("fetch_url requires a URL in args");
    console.log(JSON.stringify(await fetchUrl(url), null, 2));
    return;
  }

  throw new Error(`Unsupported action: ${opts.action}`);
}

main().catch((error) => {
  console.error(JSON.stringify({ error: error.message }, null, 2));
  process.exit(1);
});
