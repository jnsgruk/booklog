use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use tower::{Layer, Service};

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

struct RateLimiter {
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
    capacity: f64,
    refill_per_sec: f64,
}

impl RateLimiter {
    fn new(capacity: u32, window: Duration) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity: f64::from(capacity),
            refill_per_sec: f64::from(capacity) / window.as_secs_f64(),
        }
    }

    fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();

        let bucket = buckets.entry(ip).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Tower layer that applies per-IP rate limiting via a token bucket.
///
/// Falls open if the client IP cannot be determined (e.g. missing `ConnectInfo`).
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<RateLimiter>,
}

impl RateLimitLayer {
    /// Create a rate limiter allowing `requests` per minute per IP.
    pub fn per_minute(requests: u32) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(requests, Duration::from_secs(60))),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: Arc::clone(&self.limiter),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: Arc<RateLimiter>,
}

impl<S> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let limiter = Arc::clone(&self.limiter);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let ip = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip());

            if let Some(ip) = ip
                && !limiter.check(ip)
            {
                return Ok(StatusCode::TOO_MANY_REQUESTS.into_response());
            }

            inner.call(request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_requests_within_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        for _ in 0..5 {
            assert!(limiter.check(ip), "request within limit should be allowed");
        }
    }

    #[test]
    fn rejects_requests_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        for _ in 0..3 {
            assert!(limiter.check(ip));
        }
        assert!(!limiter.check(ip), "request over limit should be rejected");
    }

    #[test]
    fn tracks_ips_independently() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip1: IpAddr = "10.0.0.1".parse().unwrap();
        let ip2: IpAddr = "10.0.0.2".parse().unwrap();

        // Exhaust IP 1
        assert!(limiter.check(ip1));
        assert!(limiter.check(ip1));
        assert!(!limiter.check(ip1));

        // IP 2 should still be allowed
        assert!(limiter.check(ip2));
    }

    #[test]
    fn refills_tokens_over_time() {
        // 60 tokens per 60 seconds = 1 token per second
        let limiter = RateLimiter::new(60, Duration::from_secs(60));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Exhaust all tokens
        for _ in 0..60 {
            assert!(limiter.check(ip));
        }
        assert!(!limiter.check(ip));

        // Manually adjust the bucket's last_refill to simulate time passing
        {
            let mut buckets = limiter.buckets.lock().unwrap();
            let bucket = buckets.get_mut(&ip).unwrap();
            bucket.last_refill -= Duration::from_secs(2);
        }

        // Should have ~2 tokens refilled
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip));
    }
}
