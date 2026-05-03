import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join, resolve } from 'node:path'
import { gzipSync } from 'node:zlib'

const clientRoot = resolve(new URL('..', import.meta.url).pathname)
const pnpmBin = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const reportDir = mkdtempSync(join(tmpdir(), 'xero-perf-smoke-'))
const replayReportPath = join(reportDir, 'replay-report.json')

function runStep(label, args, options = {}) {
  console.log(`\n[perf-smoke] ${label}`)
  const result = spawnSync(pnpmBin, args, {
    cwd: clientRoot,
    env: {
      ...process.env,
      ...options.env,
    },
    stdio: 'inherit',
  })

  if (result.error) {
    throw result.error
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}

function formatBytes(bytes) {
  if (bytes < 1024) {
    return `${bytes} B`
  }

  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`
  }

  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`
}

function walkFiles(root) {
  if (!existsSync(root)) {
    return []
  }

  const files = []
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const fullPath = join(root, entry.name)
    if (entry.isDirectory()) {
      files.push(...walkFiles(fullPath))
      continue
    }

    files.push(fullPath)
  }

  return files
}

function readReplayReport() {
  if (!existsSync(replayReportPath)) {
    throw new Error(`Replay smoke report was not written at ${replayReportPath}`)
  }

  return JSON.parse(readFileSync(replayReportPath, 'utf8'))
}

function collectBundleReport() {
  const assetRoot = join(clientRoot, 'dist', 'assets')
  const chunks = walkFiles(assetRoot)
    .filter((filePath) => /\.(?:js|css)$/.test(filePath))
    .map((filePath) => {
      const bytes = statSync(filePath).size
      const contents = readFileSync(filePath)
      return {
        name: basename(filePath),
        bytes,
        gzipBytes: gzipSync(contents).byteLength,
      }
    })
    .sort((left, right) => right.bytes - left.bytes)

  const totalBytes = chunks.reduce((sum, chunk) => sum + chunk.bytes, 0)
  const totalGzipBytes = chunks.reduce((sum, chunk) => sum + chunk.gzipBytes, 0)
  const codeMirrorChunks = chunks.filter((chunk) => chunk.name.includes('codemirror'))

  return {
    totalBytes,
    totalGzipBytes,
    chunkCount: chunks.length,
    largestChunks: chunks.slice(0, 10),
    codeMirrorChunks,
  }
}

function printReplayReport(report) {
  console.log('\n[perf-smoke] Replay metrics')
  for (const [name, value] of Object.entries(report.replays)) {
    console.log(`- ${name}: ${JSON.stringify(value)}`)
  }

  console.log('\n[perf-smoke] Slowest replay tasks')
  for (const task of report.slowestTasks.slice(0, 5)) {
    console.log(`- ${task.name}: ${task.durationMs.toFixed(2)} ms`)
  }
}

function printBundleReport(bundleReport) {
  console.log('\n[perf-smoke] Bundle chunks')
  console.log(`- assets: ${bundleReport.chunkCount}`)
  console.log(`- total: ${formatBytes(bundleReport.totalBytes)} (${formatBytes(bundleReport.totalGzipBytes)} gzip)`)

  console.log('\n[perf-smoke] Largest chunks')
  for (const chunk of bundleReport.largestChunks) {
    console.log(`- ${chunk.name}: ${formatBytes(chunk.bytes)} (${formatBytes(chunk.gzipBytes)} gzip)`)
  }

  console.log('\n[perf-smoke] CodeMirror chunks')
  if (bundleReport.codeMirrorChunks.length === 0) {
    console.log('- none')
    return
  }

  for (const chunk of bundleReport.codeMirrorChunks) {
    console.log(`- ${chunk.name}: ${formatBytes(chunk.bytes)} (${formatBytes(chunk.gzipBytes)} gzip)`)
  }
}

runStep(
  'running browser-free replay tests',
  ['exec', 'vitest', 'run', 'src/performance/performance-smoke.test.tsx', '--reporter=dot'],
  { env: { XERO_PERF_SMOKE_REPORT: replayReportPath } },
)

runStep('building production frontend for chunk sizes', ['exec', 'vite', 'build', '--emptyOutDir'])

printReplayReport(readReplayReport())
printBundleReport(collectBundleReport())

console.log('\n[perf-smoke] complete')
