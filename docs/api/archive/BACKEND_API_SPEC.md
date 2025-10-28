# DeFarm Backend API Specification for Advanced Security Features

This document outlines the backend API endpoints needed to support the advanced security features implemented in Phase 2 and Phase 3.

## Overview

All three integration APIs include **local fallback mechanisms**, meaning the frontend will work with localStorage if the backend endpoints are not available. This allows for incremental implementation.

## 1. Key Exchange API

**Base URL:** `/api/v1/key-exchange`

### Public Key Directory

#### POST /public-keys
Publish a user's public key to the directory.

```json
{
  "userId": "string",
  "publicKey": "string",
  "keyType": "identity" | "encryption" | "signing",
  "algorithm": "Ed25519" | "X25519" | "RSA" | "ECDSA",
  "expiresAt": "number (optional)",
  "metadata": "object"
}
```

#### GET /public-keys/{userId}
Get a user's public key from the directory.

**Query Parameters:**
- `keyType`: Optional filter by key type

#### POST /public-keys/batch
Get public keys for multiple users.

```json
{
  "userIds": ["string"],
  "keyType": "string (optional)"
}
```

#### DELETE /public-keys/{userId}/{keyType}
Revoke a public key.

### Key Exchange Sessions

#### POST /sessions
Initiate a key exchange session.

```json
{
  "initiatorId": "string",
  "responderId": "string",
  "protocol": "ecdh" | "station-to-station" | "triple-dh" | "signal",
  "publicKey": "string",
  "metadata": "object (optional)"
}
```

#### GET /sessions/{sessionId}
Get session status.

#### POST /sessions/{sessionId}/respond
Respond to a key exchange session.

```json
{
  "publicKey": "string",
  "metadata": "object (optional)"
}
```

#### GET /users/{userId}/sessions
Get user's active sessions.

#### POST /sessions/cleanup
Cleanup expired sessions.

### Key Exchange Messages

#### POST /messages
Send a key exchange message.

```json
{
  "sessionId": "string",
  "senderId": "string",
  "messageType": "initiate" | "respond" | "confirm" | "finalize",
  "payload": "string (encrypted)",
  "signature": "string",
  "metadata": "object (optional)"
}
```

#### GET /sessions/{sessionId}/messages
Get messages for a session.

---

## 2. Audit API

**Base URL:** `/api/v1/audit`

### Event Logging

#### POST /events
Log an audit event.

```json
{
  "userId": "string",
  "eventType": "security" | "data" | "access" | "compliance" | "system" | "user",
  "action": "string",
  "resource": "string",
  "resourceId": "string (optional)",
  "outcome": "success" | "failure" | "warning" | "blocked",
  "severity": "low" | "medium" | "high" | "critical",
  "details": "object",
  "metadata": {
    "userAgent": "string (optional)",
    "ipAddress": "string (optional)",
    "location": "string (optional)",
    "deviceId": "string (optional)",
    "sessionDuration": "number (optional)"
  },
  "signature": "string (optional)",
  "compliance": {
    "gdpr": "boolean (optional)",
    "ccpa": "boolean (optional)",
    "hipaa": "boolean (optional)",
    "sox": "boolean (optional)"
  }
}
```

#### POST /events/query
Query audit events with filters.

```json
{
  "userId": "string (optional)",
  "eventTypes": ["string"] (optional),
  "actions": ["string"] (optional),
  "resources": ["string"] (optional),
  "outcomes": ["string"] (optional),
  "severities": ["string"] (optional),
  "startDate": "number (optional)",
  "endDate": "number (optional)",
  "compliance": "object (optional)",
  "limit": "number (optional, default: 100)",
  "offset": "number (optional, default: 0)",
  "sortBy": "timestamp" | "severity" | "eventType (optional)",
  "sortOrder": "asc" | "desc (optional)"
}
```

#### POST /events/sync
Sync local events to backend.

```json
{
  "events": [
    {
      "eventId": "string",
      "timestamp": "number",
      "userId": "string",
      "eventType": "string",
      "action": "string",
      "resource": "string",
      "outcome": "string",
      "severity": "string",
      "details": "object",
      "metadata": "object",
      "signature": "string (optional)",
      "compliance": "object"
    }
  ]
}
```

### Compliance Reporting

#### POST /compliance/reports
Generate a compliance report.

```json
{
  "reportType": "gdpr-data-subject" | "ccpa-consumer" | "sox-financial" | "audit-trail" | "security-incident",
  "periodStart": "number",
  "periodEnd": "number",
  "scope": {
    "userId": "string (optional)",
    "resourceTypes": ["string"] (optional),
    "eventTypes": ["string"] (optional),
    "regulations": ["string"] (optional)
  },
  "exportFormat": "json" | "csv" | "pdf" | "xml (optional)",
  "includeEvidence": "boolean (optional)"
}
```

### Security Incidents

#### POST /incidents
Create a security incident.

```json
{
  "title": "string",
  "description": "string",
  "severity": "low" | "medium" | "high" | "critical",
  "category": "unauthorized-access" | "data-breach" | "system-compromise" | "policy-violation" | "denial-of-service",
  "affectedUsers": ["string"],
  "affectedResources": ["string"],
  "relatedEventIds": ["string"] (optional),
  "confidential": "boolean (optional)"
}
```

### Dashboard & Analytics

#### GET /dashboard/metrics
Get dashboard metrics.

**Response:**
```json
{
  "totalEvents": "number",
  "eventsLast24h": "number",
  "eventsLast7d": "number",
  "securityIncidents": {
    "open": "number",
    "critical": "number",
    "resolved": "number"
  },
  "complianceStatus": {
    "gdprEvents": "number",
    "ccpaEvents": "number",
    "hipaaEvents": "number",
    "soxEvents": "number"
  },
  "topUsers": [
    {
      "userId": "string",
      "eventCount": "number",
      "riskScore": "number"
    }
  ],
  "anomalies": [
    {
      "type": "string",
      "description": "string",
      "severity": "string",
      "detectedAt": "number"
    }
  ]
}
```

### Data Export

#### POST /export/{format}
Export audit data in specified format.

**Supported formats:** `json`, `csv`, `pdf`, `xml`

**Request body:** Same as `/events/query`

---

## 3. ZK Proof API

**Base URL:** `/api/v1/zk-proofs`

### Proof Submission & Verification

#### POST /submit
Submit a proof for verification.

```json
{
  "proofType": "ownership" | "threshold" | "membership" | "range" | "timestamp" | "quality",
  "claim": "string",
  "proof": "string (JSON stringified proof data)",
  "publicInputs": "object",
  "metadata": "object",
  "verifierHints": {
    "expectedItemId": "string (optional)",
    "expectedSetId": "string (optional)",
    "expectedMerkleRoot": "string (optional)",
    "expectedMetric": "string (optional)"
  }
}
```

#### POST /{proofId}/verify
Verify a submitted proof.

```json
{
  "itemId": "string (optional)",
  "setId": "string (optional)",
  "merkleRoot": "string (optional)",
  "metric": "string (optional)",
  "allowedVerifiers": ["string"] (optional)
}
```

#### GET /{proofId}
Get proof by ID.

### Proof Search & Discovery

#### GET /search
Search proofs with filters.

**Query Parameters:**
- `authorId`: string (optional)
- `proofType`: string (optional)
- `verified`: boolean (optional)
- `startDate`: number (optional)
- `endDate`: number (optional)
- `tags`: string (comma-separated, optional)
- `limit`: number (optional, default: 50)
- `offset`: number (optional, default: 0)

#### POST /batch-verify
Verify multiple proofs at once.

```json
{
  "proofIds": ["string"],
  "verificationHints": "object (optional)"
}
```

### Circuit Templates

#### GET /circuits/templates
Get available circuit templates.

#### POST /circuits/templates
Create a custom circuit template.

```json
{
  "name": "string",
  "description": "string",
  "category": "agriculture" | "supply-chain" | "certification" | "identity" | "general",
  "inputSchema": "object",
  "outputSchema": "object",
  "constraints": ["string"],
  "complexity": "low" | "medium" | "high",
  "gasEstimate": "number (optional)",
  "version": "string"
}
```

### Verifier Management

#### POST /verifiers/register
Register as a proof verifier.

```json
{
  "name": "string",
  "description": "string",
  "supportedProofTypes": ["string"],
  "publicKey": "string",
  "stake": "number (optional)"
}
```

### Statistics & Analytics

#### GET /stats
Get verification statistics.

**Response:**
```json
{
  "totalProofs": "number",
  "verifiedProofs": "number",
  "verificationRate": "number",
  "averageVerificationTime": "number",
  "proofsByType": "object",
  "topVerifiers": [
    {
      "verifierId": "string",
      "verificationCount": "number",
      "accuracy": "number"
    }
  ],
  "recentActivity": [
    {
      "timestamp": "number",
      "action": "submitted" | "verified" | "rejected",
      "proofType": "string",
      "authorId": "string"
    }
  ]
}
```

### Data Sync

#### POST /sync
Sync local proofs to backend.

```json
{
  "proofs": [
    {
      "proofId": "string",
      "proofType": "string",
      "claim": "string",
      "proof": "string",
      "publicInputs": "object",
      "timestamp": "number",
      "metadata": "object"
    }
  ]
}
```

---

## Authentication

All endpoints require authentication via Bearer token:

```
Authorization: Bearer <token>
```

## Error Responses

All endpoints return errors in this format:

```json
{
  "success": false,
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": "object (optional)"
}
```

## Success Responses

All endpoints return success responses in this format:

```json
{
  "success": true,
  "data": "object (response data)"
}
```

## Implementation Priority

1. **High Priority (Core functionality)**
   - Key Exchange: Public key directory endpoints
   - Audit: Event logging and basic querying
   - ZK Proof: Basic submission and verification

2. **Medium Priority (Enhanced features)**
   - Key Exchange: Session management
   - Audit: Compliance reporting and dashboard
   - ZK Proof: Circuit templates and search

3. **Low Priority (Advanced features)**
   - All sync endpoints
   - Advanced analytics
   - Custom circuit creation

## Database Schema Considerations

### Key Exchange Tables
- `public_keys` (user_id, key_type, algorithm, public_key, expires_at, created_at)
- `key_exchange_sessions` (session_id, initiator_id, responder_id, protocol, status, expires_at)
- `key_exchange_messages` (message_id, session_id, sender_id, message_type, payload, signature)

### Audit Tables
- `audit_events` (id, event_id, user_id, event_type, action, resource, outcome, severity, timestamp, details, metadata, signature)
- `compliance_reports` (report_id, report_type, period_start, period_end, scope, findings, status, generated_at)
- `security_incidents` (incident_id, title, description, severity, status, category, affected_users, created_at)

### ZK Proof Tables
- `zk_proofs` (proof_id, author_id, proof_type, claim, proof_data, public_inputs, status, submitted_at, verified_at)
- `zk_circuit_templates` (circuit_id, name, description, category, input_schema, output_schema, constraints, is_active)
- `zk_verifiers` (verifier_id, name, description, supported_types, public_key, stake, registration_date)

## Notes for Backend Team

1. **Graceful Degradation**: All frontend code includes localStorage fallbacks, so endpoints can be implemented incrementally.

2. **Security**: All cryptographic operations happen client-side. The backend primarily stores public data and coordinates communication.

3. **Compliance**: Audit endpoints should support configurable retention policies and data export for regulatory compliance.

4. **Scalability**: Consider pagination for all list endpoints and indexing for search functionality.

5. **Rate Limiting**: Implement appropriate rate limiting, especially for proof verification endpoints.

6. **Monitoring**: Add monitoring for verification success rates, response times, and anomaly detection.