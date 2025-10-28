# Archived API Documentation

This directory contains historical API documentation that has been superseded by newer versions or represents feature requests that are not yet implemented.

## Files

### ITEM_SHARING_API_REQUEST.md
**Date:** 2025-09-27
**Status:** Feature Request - Not Yet Implemented
**Description:** Original frontend team request for item sharing API endpoints. This functionality is currently handled via circuits and circuit membership permissions. Direct item sharing may be implemented in a future version.

**Current Alternative:** Use circuit-based sharing:
1. Create a circuit for sharing
2. Add members to circuit
3. Push items to circuit
4. Members can pull items

### BACKEND_API_SPEC.md
**Date:** 2025-09-30
**Status:** Superseded
**Description:** Legacy backend API specification focusing on advanced security features (key exchange, end-to-end encryption, secure data sharing). This has been superseded by:
- `openapi.yaml` - Complete OpenAPI 3.0 specification
- `API_GUIDE.md` - Comprehensive bilingual guide
- Current CLAUDE.md principles

**Current Documentation:** See `../openapi.yaml` and `../API_GUIDE.md` for up-to-date API documentation.

---

## Why Archive?

These documents are preserved for historical reference but should not be used for current development:

1. **Feature Tracking:** Shows evolution of feature requests and requirements
2. **Historical Context:** Provides background on design decisions
3. **Future Reference:** May inform future feature implementations
4. **Audit Trail:** Maintains complete documentation history

## Current Documentation

For up-to-date API documentation, see:
- **[API_GUIDE.md](../API_GUIDE.md)** - Complete bilingual guide
- **[openapi.yaml](../openapi.yaml)** - OpenAPI specification
- **[GERBOV_API.md](../GERBOV_API.md)** - Client-specific documentation
- **[README.md](../README.md)** - Documentation index
