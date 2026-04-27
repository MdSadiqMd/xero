import { spawn } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const clientDir = resolve(scriptDir, '..')
const runner = resolve(clientDir, 'src-tauri', 'scripts', 'tauri-dev-runner.sh')
const tauriArgs = ['dev', ...process.argv.slice(2)]

const env = {
  ...process.env,
  CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? '4',
  CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER: runner,
  CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER: runner,
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
