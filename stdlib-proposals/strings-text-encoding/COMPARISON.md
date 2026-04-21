# String & Text Encoding Comparison

This document compares three alternatives for representing and converting strings in Opalescent. Each alternative balances the trade-offs between legacy support, modern standards, and developer experience.

## Summary Matrix

| Axis | utf8-bytes-only | multiple-encodings | codepoint-first |
|------|-----------------|--------------------|-----------------|
| **Ergonomics** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★★ |
| **Implementation effort** | Low (1-2mo) | Medium (3-5mo) | Medium (2-4mo) |
| **Extensibility** | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★★★ | ★★★★★ | ★★★★★ |

## Analysis

### utf8-bytes-only: The Modern Standard
- **Ergonomics**: Highest ergonomics due to a single, unambiguous path for encoding.
- **Error-model fit**: Perfect fit as UTF-8 decoding has well-defined failure modes.
- **Implementation effort**: Minimal implementation effort since it only requires a single codec.
- **Extensibility**: Good for modern protocols, but limited in legacy environments.

### multiple-encodings: Legacy and System Support
- **Ergonomics**: Slightly lower due to the larger API surface area (UTF-8, UTF-16, ASCII).
- **Error-model fit**: Excellent; each encoding has its own failure states.
- **Opalescent-idiom fit**: Matches the "explicit is better" philosophy.
- **Extensibility**: Highest extensibility, allowing for future encoding support if needed.

### codepoint-first: Text-Centric API
- **Ergonomics**: Very high for text analysis, but adds overhead for I/O-bound tasks.
- **Error-model fit**: Natural fit for Unicode scalar validation.
- **Implementation effort**: Medium, as it requires efficient codepoint array handling.
- **Extensibility**: Great for linguistic analysis and complex text manipulation.
