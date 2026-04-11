# CareOps Security Model

## 1. Authentication

### Flow

1. User submits `username` + `password` to `POST /api/auth/login`
2. Backend looks up the user and verifies credentials against Argon2id hash
3. On success: creates a session record, issues a JWT containing `user_id`, `session_id`, `org_id`
4. Client stores the JWT and includes it as `Authorization: Bearer <token>` on subsequent requests
5. On logout (`POST /api/auth/logout`): session is marked as revoked in the database

### Password Storage

- Algorithm: Argon2id (via the `argon2` crate with default parameters)
- Salts: randomly generated per password (SaltString from OsRng)
- Plaintext passwords are never stored, logged, or returned in API responses

### Session Management

- Sessions are stored in the `sessions` table with a SHA-256 hash of the JWT token
- Each session has an `expires_at` timestamp and optional `revoked_at`
- Token validation checks: JWT signature + expiration + session not revoked
- Token revocation is immediate on logout

### Account Lockout

- Failed login attempts are tracked in `user_credentials.failed_attempts`
- After 5 consecutive failures, account is locked for 15 minutes (`locked_until`)
- Successful login resets the failure counter

### Implementation Files

| Component | File |
|-----------|------|
| Password hashing | `backend/src/application/auth_service.rs` (hash_password, verify_password) |
| JWT issuance/validation | `backend/src/application/auth_service.rs` (login, validate_token) |
| Session storage | `backend/migrations/20240102000000_security_rbac_audit.sql` (sessions table) |
| Auth request guard | `backend/src/api/guards/mod.rs` (AuthenticatedUser) |
| Claims type | `backend/src/domain/auth_types.rs` (JwtClaims) |

---

## 2. RBAC Model

### Architecture

The RBAC system uses three layers of permission control:

```
User -> [User Roles] -> Role -> [Role Permissions] -> Permission
```

Permissions are categorized by enforcement point:

| Category | Purpose | Example |
|----------|---------|---------|
| `menu` | Frontend sidebar/navigation visibility | `menu.dashboard`, `menu.admin` |
| `action` | Button/action visibility and backend enforcement | `action.users.create`, `action.billing.generate` |
| `api` | API endpoint authorization | `api.users.read`, `api.billing.write` |
| `data` | Data-scope access rules | (enforced via data_scopes table) |

### Default Roles

| Role | Description | Key Permissions |
|------|-------------|-----------------|
| **System Administrator** | Full access to all system features | All permissions |
| **Operations Manager** | Operational oversight across delivery and billing | Catalog, plans, delivery, billing, scoring management |
| **Billing Specialist** | Billing and payment processing | Billing, payments, plan/delivery read |
| **Coach/Clinician** | Service delivery logging | Plans read, delivery read/write |
| **QA Reviewer** | Quality scoring and review | Delivery read, scoring read/write, reports |
| **Auditor** | Read-only audit and compliance | Audit log read, reports read/export |

### Permission Enforcement Points

1. **Frontend navigation**: Sidebar links only render for permitted menu items (`components/sidebar.rs`)
2. **Frontend actions**: Buttons/actions conditionally render based on permission set (`state/mod.rs: has_permission()`)
3. **Backend API guard**: `AuthenticatedUser` request guard rejects unauthenticated requests with 401
4. **Backend route-level**: `guards::require_permission()` checks permission codes before handler logic
5. **Backend object-level**: `guards::require_data_scope()` checks org/department/project access

Frontend visibility is advisory only. Backend enforcement is mandatory and independent.

### Implementation Files

| Component | File |
|-----------|------|
| Permission definitions | `backend/src/domain/auth_policy.rs` |
| Role-permission matrix | `backend/src/domain/auth_policy.rs` (default_role_permissions) |
| Permission cache | `backend/src/infrastructure/permission_cache/mod.rs` |
| Route-level guards | `backend/src/api/guards/mod.rs` |
| Frontend permission check | `frontend/src/state/mod.rs` (AuthState::has_permission) |
| Frontend sidebar | `frontend/src/components/sidebar.rs` |

---

## 3. Data-Scope Model

### Scope Hierarchy

```
Organization
  └── Department
       └── Project
```

Each user has explicit data-scope grants stored in `user_data_scopes`:

| Field | Purpose |
|-------|---------|
| `org_id` | Required: which organization's data the user can access |
| `department_id` | Optional: restricts to a specific department |
| `project_id` | Optional: restricts to a specific project |
| `access_level` | `read`, `write`, or `admin` |

### Scope Resolution Rules

1. An **org-level** scope (department_id=NULL) grants access to all departments and projects within that org
2. A **department-level** scope grants access to all projects within that department
3. A **project-level** scope grants access only to that specific project
4. Access level is hierarchical: `admin` > `write` > `read`
5. **System Administrator** role bypasses all scope checks

### Enforcement

Data-scope is enforced at the API level via `guards::require_data_scope()`:

```rust
guards::require_data_scope(
    perm_cache, &user.user_id,
    &org_id, department_id.as_deref(), project_id.as_deref(),
    "write" // required access level
).await?;
```

This is called before any data-modifying operation to ensure the user has appropriate scope access.

### Implementation Files

| Component | File |
|-----------|------|
| Scope table | `backend/migrations/20240102000000_security_rbac_audit.sql` (user_data_scopes) |
| Scope checking | `backend/src/infrastructure/permission_cache/mod.rs` (check_data_scope) |
| Guard helper | `backend/src/api/guards/mod.rs` (require_data_scope) |
| Scope CRUD | `backend/src/application/user_service.rs` (assign_scope, revoke_scope) |

---

## 4. Permission Cache / Versioning

### Design

The permission cache prevents hitting the database on every API request while ensuring permission changes take effect within 30 seconds.

```
┌──────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  API Request  │────▶│  Permission     │────▶│  permission_    │
│               │     │  Cache (in-     │     │  version table  │
│  check perm   │     │  process,       │     │  (single row)   │
│               │     │  RwLock)        │     │                 │
└──────────────┘     └──────────────────┘     └─────────────────┘
```

### Mechanism

1. Every TTL interval (max 30 seconds), the cache reads the global `version` from `permission_version`
2. If the version changed since last check, the entire cache is cleared
3. Individual user entries are loaded from DB on cache miss
4. Any permission/role/scope change increments the global version counter
5. Thread safety via `parking_lot::RwLock` for low-contention reads

### Version Invalidation

These operations increment the permission version (trigger cache refresh):
- Role assignment/revocation (`user_service.assign_role/revoke_role`)
- Permission grant/revocation (`role_service.assign_permission/revoke_permission`)
- Data-scope grant/revocation (`user_service.assign_scope/revoke_scope`)

### Timing Guarantee

- Cache TTL is capped at 30 seconds in code: `Duration::from_secs(ttl_seconds.min(30))`
- Worst case: a permission change is visible to all requests within 30 seconds
- The change is immediately visible to the user whose cache entry was invalidated

### Implementation Files

| Component | File |
|-----------|------|
| Cache implementation | `backend/src/infrastructure/permission_cache/mod.rs` |
| Version table | `backend/migrations/20240102000000_security_rbac_audit.sql` (permission_version) |
| Cache invalidation calls | `backend/src/application/user_service.rs`, `role_service.rs` |

---

## 5. Encryption at Rest

### Approach

Sensitive fields are encrypted using AES-256-GCM before storage in MySQL. The encryption key is derived from environment-provided key material using SHA-256.

### Encrypted Format

```
base64( 12-byte-nonce || ciphertext || authentication-tag )
```

Each encryption operation generates a random 12-byte nonce, ensuring identical plaintext produces different ciphertext.

### Key Management

- Key material is provided via the `ENCRYPTION_KEY` environment variable
- SHA-256 is used to derive a 256-bit key from the key material
- The docker-compose.yml includes a dev-safe default key (must be changed for production)
- The `EncryptionService` struct redacts the key in Debug output

### Encrypted Fields

| Table | Field | Purpose |
|-------|-------|---------|
| `client_plans` | `client_identifier_enc` | Encrypted client identifier |
| `client_plans` | `notes_enc` | Encrypted sensitive notes |
| `delivery_entries` | `notes` | Encrypted when containing sensitive content |
| `quality_scores` | `notes` | Encrypted when containing sensitive content |

### Masking

The `EncryptionService::mask()` function provides safe display of sensitive values (e.g., `"12*****89"`). Used in API responses and log output to prevent sensitive data leakage.

### Implementation Files

| Component | File |
|-----------|------|
| Encryption service | `backend/src/infrastructure/encryption/mod.rs` |
| Encryption tests | `backend/src/infrastructure/encryption/mod.rs` (tests module) |
| Key configuration | `backend/src/config/mod.rs`, `docker-compose.yml` |

---

## 6. Audit Logging

### Strategy

All security-sensitive actions are logged to an immutable `audit_logs` table. The audit service is fire-and-forget: audit write failures are logged but never block the calling operation.

### Logged Events

| Category | Actions |
|----------|---------|
| **Authentication** | `auth.login.success`, `auth.login.failed`, `auth.logout` |
| **User Management** | `user.created`, `user.updated`, `user.deactivated`, `user.password.changed` |
| **Role Changes** | `role.created`, `role.assigned`, `role.revoked` |
| **Permission Changes** | `permission.granted`, `permission.revoked` |
| **Scope Changes** | `scope.granted`, `scope.revoked` |
| **Organization** | `org.created`, `org.updated`, `department.created`, `project.created` |
| **Configuration** | `config.changed` |

### Audit Record Structure

```json
{
  "id": "uuid",
  "timestamp": "2024-01-01T00:00:00",
  "user_id": "actor-uuid",
  "action": "user.created",
  "resource_type": "user",
  "resource_id": "target-uuid",
  "org_id": "org-uuid",
  "details": {"username": "newuser", "email": "..."},
  "ip_address": "192.168.1.1",
  "trace_id": "request-trace-id"
}
```

### What Is NOT Logged

- Passwords (neither plaintext nor hashed)
- Full client identifiers (masked if included)
- Decrypted sensitive notes
- Encryption keys or secrets

### Querying

Audit logs are queryable via `GET /api/audit/` with filters:
- `user_id`: filter by acting user
- `action`: filter by action prefix (e.g., `auth.` for all auth events)
- `resource_type`: filter by resource type
- Pagination via `limit` and `offset`

Access requires the `api.audit.read` permission (Auditor and System Administrator roles).

### Implementation Files

| Component | File |
|-----------|------|
| Audit service | `backend/src/infrastructure/audit/mod.rs` |
| Action constants | `backend/src/infrastructure/audit/mod.rs` (actions module) |
| Audit API | `backend/src/api/audit_api/mod.rs` |
| Audit table | `backend/migrations/20240102000000_security_rbac_audit.sql` |

---

## 7. Security Configuration

### Environment Variables

| Variable | Purpose | Default (Dev) |
|----------|---------|---------------|
| `JWT_SECRET` | JWT signing key | `careops-dev-jwt-secret-...` |
| `ENCRYPTION_KEY` | AES-256-GCM key material | `careops-dev-encryption-key-...` |
| `SESSION_TTL_HOURS` | JWT/session expiration | `24` |
| `DATABASE_URL` | MySQL connection string | `mysql://careops_user:careops_pass@db:3306/careops` |

### Production Checklist

- [ ] Change `JWT_SECRET` to a cryptographically random 64+ character string
- [ ] Change `ENCRYPTION_KEY` to a cryptographically random 64+ character string
- [ ] Set `SESSION_TTL_HOURS` to appropriate value (e.g., 8 for business hours)
- [ ] Change MySQL passwords in docker-compose.yml
- [ ] Enable TLS termination in front of the backend
- [ ] Review and restrict CORS allowed origins
- [ ] Disable demo user seeds or change default passwords
