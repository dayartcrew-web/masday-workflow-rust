// ============================================================
// Sliding window rate limiter — per-client, configurable per-route
// ============================================================

interface RateBucket {
  count: number;
  resetAt: number;
}

export interface RateLimitConfig {
  /** Window duration in milliseconds */
  windowMs: number;
  /** Max requests per window */
  max: number;
}

export interface RateLimitResult {
  allowed: boolean;
  remaining: number;
  resetAt: number;
  retryAfter?: number;
}

const DEFAULT_CONFIG: RateLimitConfig = {
  windowMs: 60_000,
  max: 100,
};

export class RateLimiter {
  private buckets: Map<string, RateBucket> = new Map();
  private defaultConfig: RateLimitConfig;
  private routeConfigs: Map<string, RateLimitConfig> = new Map();
  private cleanupCounter = 0;

  constructor(defaultConfig?: Partial<RateLimitConfig>) {
    this.defaultConfig = { ...DEFAULT_CONFIG, ...defaultConfig };
  }

  /** Set a per-route rate limit configuration */
  setRouteConfig(routePrefix: string, config: RateLimitConfig): void {
    this.routeConfigs.set(routePrefix, config);
  }

  /** Check if a request is allowed */
  check(key: string, route?: string): RateLimitResult {
    const now = Date.now();
    const config = this.getEffectiveConfig(route);

    let bucket = this.buckets.get(key);

    // Reset bucket if window has expired
    if (!bucket || now >= bucket.resetAt) {
      bucket = { count: 0, resetAt: now + config.windowMs };
      this.buckets.set(key, bucket);
    }

    bucket.count++;
    const remaining = Math.max(0, config.max - bucket.count);

    // Periodic cleanup of expired buckets
    this.cleanupCounter++;
    if (this.cleanupCounter % 200 === 0) {
      this.cleanup(now);
    }

    const allowed = bucket.count <= config.max;
    const retryAfter = allowed ? undefined : Math.ceil((bucket.resetAt - now) / 1000);

    return {
      allowed,
      remaining,
      resetAt: bucket.resetAt,
      retryAfter,
    };
  }

  /** Get the effective rate limit config for a route */
  private getEffectiveConfig(route?: string): RateLimitConfig {
    if (!route) return this.defaultConfig;

    // Find the most specific matching route config
    for (const [prefix, config] of this.routeConfigs) {
      if (route.startsWith(prefix)) return config;
    }

    return this.defaultConfig;
  }

  /** Remove expired buckets */
  private cleanup(now: number): void {
    for (const [key, bucket] of this.buckets) {
      if (now >= bucket.resetAt) {
        this.buckets.delete(key);
      }
    }
  }

  /** Reset all rate limit state */
  reset(): void {
    this.buckets.clear();
    this.cleanupCounter = 0;
  }
}
