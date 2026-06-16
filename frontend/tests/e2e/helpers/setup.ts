import { execSync, spawn } from 'child_process'
import { existsSync, writeFileSync } from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = path.resolve(__dirname, '../../../../')
const PG_CONTAINER_NAME = 'batchwise-e2e-pg'
const PG_PORT = 5434
export const BACKEND_PORT = 8082
const STATE_FILE = '/tmp/batchwise-e2e-state.json'
const JWT_SECRET = 'e2e-test-secret-32-bytes-longXXXX'

async function waitForUrl(url: string, maxMs = 30_000): Promise<void> {
  const deadline = Date.now() + maxMs
  while (Date.now() < deadline) {
    try {
      const res = await fetch(url)
      if (res.ok) return
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 500))
  }
  throw new Error(`Timed out waiting for ${url}`)
}

export default async function globalSetup() {
  let dbURL = process.env.E2E_DATABASE_URL
  let startedPg = false

  if (!dbURL) {
    // Clean up any leftover container
    try {
      execSync(`podman rm -f ${PG_CONTAINER_NAME}`, { stdio: 'ignore' })
    } catch {}

    execSync(
      `podman run -d --name ${PG_CONTAINER_NAME}` +
        ` -e POSTGRES_USER=batchwise -e POSTGRES_PASSWORD=batchwise -e POSTGRES_DB=batchwise` +
        ` -p ${PG_PORT}:5432 --rm docker.io/library/postgres:16`,
      { stdio: 'inherit' },
    )
    dbURL = `postgresql://batchwise:batchwise@localhost:${PG_PORT}/batchwise?sslmode=disable`
    startedPg = true

    // Wait for postgres to accept connections
    await waitForUrl(`http://localhost:${BACKEND_PORT}/healthz`, 5_000).catch(() => {})
    await new Promise((r) => setTimeout(r, 3_000))
  }

  // Build the backend binary if not already present (CI pre-builds it)
  const binaryPath = path.join(REPO_ROOT, 'bin', 'batchwise')
  if (!existsSync(binaryPath)) {
    execSync('make build', { cwd: REPO_ROOT, stdio: 'inherit' })
  }

  const backendEnv: NodeJS.ProcessEnv = {
    ...process.env,
    DATABASE_URL: dbURL,
    JWT_SECRET,
    BOOTSTRAP_REGISTRATION_ENABLED: 'true',
    HTTP_PORT: String(BACKEND_PORT),
    APP_ENV: 'development',
    CORS_ORIGIN: 'http://localhost:5173',
    LOG_LEVEL: 'error',
  }

  const backend = spawn(path.join(REPO_ROOT, 'bin', 'batchwise'), [], {
    env: backendEnv,
    cwd: REPO_ROOT,
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: false,
  })

  backend.stdout?.on('data', (d: Buffer) => process.stdout.write(d))
  backend.stderr?.on('data', (d: Buffer) => process.stderr.write(d))

  // Wait for backend healthz
  await waitForUrl(`http://localhost:${BACKEND_PORT}/healthz`)

  writeFileSync(
    STATE_FILE,
    JSON.stringify({ backendPid: backend.pid, startedPg }),
  )

  // Make backend port available to test files via env
  process.env.E2E_BACKEND_PORT = String(BACKEND_PORT)
  process.env.E2E_JWT_SECRET = JWT_SECRET
}
