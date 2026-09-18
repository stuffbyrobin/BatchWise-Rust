//! Shared rate-limit windows in Redis.
//!
//! An in-memory window only limits the instance that saw the request, so with
//! more than one replica a caller gets the limit once per replica. When
//! `REDIS_URL` is set, every instance runs the same sliding-window script
//! against one Redis and the limit belongs to the deployment. Keys are
//! `<prefix>:rl:<limiter>:<caller>` and expire with their window.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::{RedisError, Script};
use tokio::sync::OnceCell;
use tokio::time::timeout;
use uuid::Uuid;

/// Sliding-window check, run inside Redis so concurrent instances cannot race.
/// `KEYS[1]` is the window; `ARGV` is the window in milliseconds, the limit,
/// whether to record this call, and a unique member id. Returns 0 when the call
/// is allowed, otherwise the seconds to wait. The clock is Redis's own, so
/// instances with skewed clocks still agree.
const SLIDING_WINDOW: &str = r"
local window = tonumber(ARGV[1])
local limit = tonumber(ARGV[2])
local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000 + math.floor(tonumber(time[2]) / 1000)
redis.call('ZREMRANGEBYSCORE', KEYS[1], '-inf', now - window)
if redis.call('ZCARD', KEYS[1]) >= limit then
  local oldest = redis.call('ZRANGE', KEYS[1], 0, 0, 'WITHSCORES')
  local retry = math.ceil((tonumber(oldest[2]) + window - now) / 1000)
  if retry < 1 then retry = 1 end
  return retry
end
if ARGV[3] == '1' then
  redis.call('ZADD', KEYS[1], now, ARGV[4])
  redis.call('PEXPIRE', KEYS[1], window)
end
return 0
";

/// How long to stay quiet after logging that Redis is unreachable.
const WARNING_INTERVAL: Duration = Duration::from_secs(60);
/// A rate-limit check must never add noticeable latency, so a slow Redis is
/// abandoned and the in-memory window answers instead.
const CHECK_TIMEOUT: Duration = Duration::from_millis(250);
/// After a failure, skip Redis for this long: during an outage every request
/// would otherwise wait for the timeout above.
const COOLDOWN: Duration = Duration::from_secs(5);

/// The Redis holding shared rate-limit windows.
pub struct RedisRateLimits {
    client: redis::Client,
    prefix: String,
    script: Script,
    conn: OnceCell<ConnectionManager>,
    last_failure: Mutex<Option<Instant>>,
    last_warning: Mutex<Option<Instant>>,
}

impl std::fmt::Debug for RedisRateLimits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The URL can carry a password, so only the key prefix is shown.
        f.debug_struct("RedisRateLimits")
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl RedisRateLimits {
    /// Parses the URL. The connection is made on first use and re-tried after a
    /// failure, so the app starts even when Redis is not up yet.
    pub fn new(url: &str, prefix: &str) -> Result<Self, RedisError> {
        Ok(Self {
            client: redis::Client::open(url)?,
            prefix: prefix.to_string(),
            script: Script::new(SLIDING_WINDOW),
            conn: OnceCell::new(),
            last_failure: Mutex::new(None),
            last_warning: Mutex::new(None),
        })
    }

    /// Counts a call against `limiter`'s window for `caller`, or only reads the
    /// window when `record` is false. Returns the seconds to wait when it is
    /// full, or an error when Redis is slow, unreachable or still cooling down
    /// after a failure; the caller then falls back to its in-memory window.
    pub async fn hit(
        &self,
        limiter: &str,
        caller: &str,
        window: Duration,
        limit: usize,
        record: bool,
    ) -> Result<Option<u64>, RedisError> {
        if self.cooling_down() {
            return Err(unavailable(
                "shared rate limit is cooling down after a failure",
            ));
        }
        match timeout(
            CHECK_TIMEOUT,
            self.run(limiter, caller, window, limit, record),
        )
        .await
        {
            Ok(Ok(retry)) => Ok(retry),
            Ok(Err(e)) => {
                self.note_failure();
                Err(e)
            }
            Err(_) => {
                self.note_failure();
                Err(unavailable("shared rate limit timed out"))
            }
        }
    }

    async fn run(
        &self,
        limiter: &str,
        caller: &str,
        window: Duration,
        limit: usize,
        record: bool,
    ) -> Result<Option<u64>, RedisError> {
        let mut conn = self.connection().await?;
        let retry: i64 = self
            .script
            .key(format!("{}:rl:{limiter}:{caller}", self.prefix))
            .arg(window.as_millis() as i64)
            .arg(limit as i64)
            .arg(i32::from(record))
            .arg(Uuid::new_v4().to_string())
            .invoke_async(&mut conn)
            .await?;
        Ok((retry > 0).then_some(retry as u64))
    }

    fn cooling_down(&self) -> bool {
        self.last_failure
            .lock()
            .expect("redis failure mutex")
            .is_some_and(|at| at.elapsed() < COOLDOWN)
    }

    fn note_failure(&self) {
        *self.last_failure.lock().expect("redis failure mutex") = Some(Instant::now());
    }

    async fn connection(&self) -> Result<ConnectionManager, RedisError> {
        // One quick attempt: reconnection backoff belongs to the cooldown above,
        // not to a request that is waiting on a rate-limit check.
        let config = ConnectionManagerConfig::new()
            .set_number_of_retries(0)
            .set_connection_timeout(Some(CHECK_TIMEOUT))
            .set_response_timeout(Some(CHECK_TIMEOUT));
        self.conn
            .get_or_try_init(|| {
                ConnectionManager::new_with_config(self.client.clone(), config.clone())
            })
            .await
            .cloned()
    }

    /// Logs an unreachable Redis at most once a minute: during an outage every
    /// request fails, and the limiter falls back to its in-memory window.
    pub fn warn(&self, limiter: &str, error: &RedisError) {
        let mut last = self.last_warning.lock().expect("redis warning mutex");
        if last.is_none_or(|at| at.elapsed() > WARNING_INTERVAL) {
            tracing::warn!(
                limiter,
                error = %error,
                "shared rate limit unavailable; limiting per instance"
            );
            *last = Some(Instant::now());
        }
    }
}

/// A stand-in error for a Redis that is too slow or is cooling down.
fn unavailable(reason: &'static str) -> RedisError {
    RedisError::from(std::io::Error::new(std::io::ErrorKind::TimedOut, reason))
}
