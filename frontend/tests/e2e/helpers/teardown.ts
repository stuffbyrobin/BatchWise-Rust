import { execSync } from 'child_process'
import { readFileSync, existsSync, unlinkSync } from 'fs'

const STATE_FILE = '/tmp/batchwise-e2e-state.json'

export default async function globalTeardown() {
  if (!existsSync(STATE_FILE)) return

  let state: { backendPid?: number; startedPg?: boolean }
  try {
    state = JSON.parse(readFileSync(STATE_FILE, 'utf8'))
  } catch {
    return
  }

  // Kill the backend process
  if (state.backendPid) {
    try {
      process.kill(state.backendPid, 'SIGTERM')
    } catch {}
  }

  // Remove the postgres container we started
  if (state.startedPg) {
    try {
      execSync('podman rm -f batchwise-e2e-pg', { stdio: 'ignore' })
    } catch {}
  }

  try {
    unlinkSync(STATE_FILE)
  } catch {}
}
