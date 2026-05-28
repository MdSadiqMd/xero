import { spawn } from 'node:child_process'
import { homedir } from 'node:os'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { loadRootDotenv } from '../../scripts/lib/env.mjs'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const clientDir = resolve(scriptDir, '..')
const repoRoot = resolve(clientDir, '..')
const runner = resolve(clientDir, 'src-tauri', 'scripts', 'tauri-dev-runner.sh')
const devTauriConfig = resolve(clientDir, 'src-tauri', 'tauri.dev.conf.json')
const devAppDataDir = defaultAppDataDir('dev.sn0w.xero')
const tauriArgs = ['dev', '--config', devTauriConfig, ...process.argv.slice(2)]
const rootEnv = loadRootDotenv(repoRoot)

const env = {
  ...rootEnv,
  CARGO_BUILD_JOBS: rootEnv.CARGO_BUILD_JOBS ?? '4',
  CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER: runner,
  CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER: runner,
  XERO_APP_DATA_DIR: rootEnv.XERO_APP_DATA_DIR ?? devAppDataDir,
}

const command = process.platform === 'win32' ? 'tauri.cmd' : 'tauri'
const child = spawn(command, tauriArgs, {
  cwd: clientDir,
  env,
  shell: process.platform === 'win32',
  stdio: 'inherit',
})

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal)
    return
  }

  process.exit(code ?? 1)
})

child.on('error', (error) => {
  console.error(`Failed to start Tauri dev: ${error.message}`)
  process.exit(1)
})

function defaultAppDataDir(directoryName) {
  if (process.platform === 'darwin') {
    return resolve(homedir(), 'Library', 'Application Support', directoryName)
  }
  if (process.platform === 'win32') {
    return resolve(process.env.APPDATA || process.env.LOCALAPPDATA || homedir(), directoryName)
  }
  return resolve(process.env.XDG_DATA_HOME || resolve(homedir(), '.local', 'share'), directoryName)
}
