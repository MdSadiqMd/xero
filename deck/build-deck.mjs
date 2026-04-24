import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";
import fs from "node:fs/promises";

const runtimeRequire = createRequire(
  "/Users/sn0w/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/.codex-require.cjs",
);

const { chromium } = runtimeRequire("playwright");
const sharp = runtimeRequire("sharp");

const __filename = fileURLToPath(import.meta.url);
const deckDir = path.dirname(__filename);
const htmlPath = path.join(deckDir, "xero-helius-pitch-deck.html");
const pdfPath = path.join(deckDir, "xero-helius-pitch-deck.pdf");
const previewDir = path.join(deckDir, "previews");
const reportPath = path.join(deckDir, "render-report.json");

await fs.mkdir(previewDir, { recursive: true });
for (const file of await fs.readdir(previewDir)) {
  if (file.endsWith(".png")) {
    await fs.unlink(path.join(previewDir, file));
  }
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1600, height: 900 },
  deviceScaleFactor: 1,
});

await page.goto(pathToFileURL(htmlPath).href, { waitUntil: "networkidle" });
await page.evaluate(async () => {
  await document.fonts.ready;
  await Promise.all(
    Array.from(document.images)
      .filter((img) => !img.complete)
      .map(
        (img) =>
          new Promise((resolve, reject) => {
            img.onload = resolve;
            img.onerror = reject;
          }),
      ),
  );
});

const slideHandles = await page.$$(".slide");
const previews = [];

for (let index = 0; index < slideHandles.length; index += 1) {
  const fileName = `slide-${String(index + 1).padStart(2, "0")}.png`;
  const previewPath = path.join(previewDir, fileName);
  await slideHandles[index].screenshot({ path: previewPath });
  previews.push(previewPath);
}

await page.pdf({
  path: pdfPath,
  width: "1600px",
  height: "900px",
  printBackground: true,
  preferCSSPageSize: true,
  margin: { top: "0px", right: "0px", bottom: "0px", left: "0px" },
});

await browser.close();

const checks = [];
for (const previewPath of previews) {
  const image = sharp(previewPath);
  const metadata = await image.metadata();
  const stats = await image.stats();
  const channelMeans = stats.channels.slice(0, 3).map((channel) => channel.mean);
  const mean = channelMeans.reduce((sum, value) => sum + value, 0) / channelMeans.length;
  const nonBlank = mean > 8 && mean < 248 && stats.entropy > 1.2;
  const correctSize = metadata.width === 1600 && metadata.height === 900;
  checks.push({
    file: path.relative(deckDir, previewPath),
    width: metadata.width,
    height: metadata.height,
    mean: Number(mean.toFixed(2)),
    entropy: Number(stats.entropy.toFixed(3)),
    passed: Boolean(nonBlank && correctSize),
  });
}

const pdfStat = await fs.stat(pdfPath);
const report = {
  generatedAt: new Date().toISOString(),
  html: path.relative(deckDir, htmlPath),
  pdf: path.relative(deckDir, pdfPath),
  pdfBytes: pdfStat.size,
  slideCount: slideHandles.length,
  previews: checks,
  passed: checks.length === 10 && checks.every((check) => check.passed) && pdfStat.size > 100_000,
};

await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");

console.log(JSON.stringify(report, null, 2));
