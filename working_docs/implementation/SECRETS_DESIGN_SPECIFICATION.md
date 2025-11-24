# Secret Data Model & Encrypted Storage - Design Specification

## Document Overview

This specification defines the complete design for Descartes Phase 2's encrypted secret storage system. It covers data models, encryption schemes, database schema, access control, and security implementations.

---

## 1. Data Model Design

### 1.1 Secret Metadata Structure

All secret metadata is **always unencrypted** and stored for quick queries:

```rust
pub struct SecretMetadata {
    pub id: Uuid,                        // Unique identifier
    pub name: String,                   // UNIQUE, searchable
    pub secret_type: SecretType,        // Type classification
    pub description: Option<String>,    // Human-readable purpose
    pub service: Option<String>,        // Application/service name
    pub current_version: u32,           // Current active version
    pub created_at: DateTime<Utc>,      // Creation timestamp
    pub updated_at: DateTime<Utc>,      // Last modification
    pub last_accessed_at: Option<DateTime<Utc>>,  // Last read time
    pub expires_at: Option<DateTime<Utc>>,        // Optional expiration
    pub tags: Vec<String>,              // Search/organization tags
    pub is_active: bool,                // Active/inactive status
}
```

**Rationale for Unencrypted Metadata:**
1. Enables efficient searching without decryption
2. Allows user to remember secret purposes
3. Metadata alone doesn't expose sensitive values
4. Reduces per-access decryption overhead

### 1.2 Encrypted Secret Data

Only the actual secret VALUE is encrypted:

```rust
pub struct EncryptedSecretData {
    pub ciphertext: Vec<u8>,        // Encrypted secret value
    pub nonce: Vec<u8>,             // 96-bit unique nonce for GCM
    pub tag: Vec<u8>,               // 128-bit GCM authentication tag
    pub version: u8,                // Encryption scheme version
}
```

**Encryption Properties:**
- **Algorithm**: AES-256-GCM (AEAD)
- **Key Size**: 256 bits (32 bytes)
- **Nonce Size**: 96 bits (12 bytes) - MUST be unique per encryption
- **Tag Size**: 128 bits (16 bytes) - validates authenticity
- **Version**: Current = 1 (allows future algorithm migration)

### 1.3 Secret Types

```rust
pub enum SecretType {
    ApiKey,                 // External API keys (GitHub, AWS, etc.)
    OAuthToken,             // OAuth2 tokens, bearer tokens
    DatabasePassword,       // DB credentials
    PrivateKey,             // SSH, crypto private keys
    Custom,                 // Generic/unclassified
}
```

**Type-Specific Considerations:**
- **API Keys**: Often have expiration dates
- **OAuth Tokens**: May have refresh tokens, scopes
- **DB Passwords**: May be rotated separately from DB changes
- **Private Keys**: Should have strongest protections, no expiration
- **Custom**: User-defined purpose

### 1.4 Version Control

Secrets support versioning for rotation tracking:

```rust
pub struct SecretVersion {
    pub secret_id: Uuid,
    pub version: u32,                   // Version number
    pub encrypted_data: EncryptedSecretData,
    pub created_at: DateTime<Utc>,      // When version created
    pub rotated_at: Option<DateTime<Utc>>,  // When rotated away
    pub is_valid: bool,                 // Can still be used?
}
```

**Version Lifecycle:**
1. Version created when secret is created
2. New version created on rotation
3. Old versions remain accessible for emergency recovery (30+ days)
4. Eventually marked invalid after retention period expires

---

## 2. Encryption Scheme Details

### 2.1 AES-256-GCM (Galois/Counter Mode)

**Why AES-256-GCM?**

| Property | Benefit |
|----------|---------|
| Authenticated Encryption (AEAD) | Provides both confidentiality AND authenticity |
| 256-bit key | Post-quantum resistant key size |
| GCM mode | Hardware-accelerated on modern CPUs (AES-NI) |
| Nonce-based | No state management needed |
| Fast | ~cycles per byte on modern hardware |

**Encryption Flow:**

```
┌─────────────┐
│ Secret Value│  32-byte key  96-bit random nonce
└──────┬──────┘       │                │
       │              ▼                ▼
       │        ┌─────────────────────────────┐
       └───────►│  AES-256-GCM Encryption    │
                └──────────┬──────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
    ┌──────────┐     ┌────────┐      ┌──────┐
    │Ciphertext│     │ Nonce  │      │ Tag  │
    │(variable)│     │(12 bytes)     │(16B) │
    └──────────┘     └────────┘      └──────┘
```

### 2.2 Nonce/IV Management

**Critical Security Property**: Each encryption MUST use a unique nonce!

```
DANGER: Nonce Reuse with Same Key = Complete Compromise
        Reusing (key, nonce) allows attacker to derive plaintext from ciphertexts
```

**Implementation:**
- Generate random 96-bit (12-byte) nonce for EVERY encryption
- Store nonce with ciphertext (it's public)
- Verify nonce uniqueness on decrypt
- Use thread-safe RNG (rand::ThreadRng)

**Nonce Storage in Database:**
```sql
INSERT INTO secret_values (
    secret_id, version, ciphertext, nonce, tag, encryption_version
) VALUES (?, ?, ?, ?, ?, 1);
-- nonce is always 12 bytes, publicly stored
```

### 2.3 Authentication Tag Verification

GCM provides authentication tags that detect:
- Ciphertext corruption
- Nonce tampering
- Accidental data loss
- Intentional tampering attempts

**On Decryption:**
```
IF authentication_tag_verify(ciphertext, tag, nonce, key) FAILS:
    REJECT decrypt request
    Log: "Decryption failed - authentication tag mismatch"
    Increment failed_decryption_counter
    Consider rate limiting on multiple failures
```

### 2.4 Key Size Justification

**256-bit key (32 bytes):**
- AES-128 (16 bytes): 2^128 security, probably sufficient for most uses
- AES-192 (24 bytes): 2^192 security, rarely used
- AES-256 (32 bytes): 2^256 security, future-proof, minimal performance cost

Choose AES-256 because:
1. Only ~10% slower than AES-128
2. Protects against quantum computing threats
3. Matches key derivation output size
4. NIST recommended for long-term secrets

---

## 3. Key Derivation

### 3.1 Key Derivation Function (KDF) Selection

Two options supported, both with ~100ms computation:

#### Option A: PBKDF2-HMAC-SHA256 (Simpler)

```
Master Key = PBKDF2-HMAC-SHA256(
    password = user_master_password,
    salt = 256-bit random salt,
    iterations = 480,000,  # NIST 2024 minimum for sensitive data
    output_length = 32,    # 256 bits for AES-256
    hash_function = SHA256
)
```

**Algorithm Steps:**
```
1. PRF = HMAC-SHA256
2. For iteration 1 to 480,000:
       U_i = PRF(password, salt || i)
       result = result XOR U_i
3. return first 32 bytes of result
```

**Security:**
- 480,000 iterations = ~100ms on 2024 hardware
- Makes brute-force attack cost: 480,000 × password attempts
- Memory cost: negligible

**Use When:**
- Simplicity is important
- You need compatibility with other systems
- Performance is more important than GPU-resistance

#### Option B: Argon2id (Recommended)

```
Master Key = Argon2id(
    password = user_master_password,
    salt = 256-bit random salt,
    memory = 19 MiB (19,456 KiB),
    parallelism = 1,
    iterations = 2,
    output_length = 32
)
```

**Algorithm Properties:**
- **Memory-hard**: Requires significant RAM (19 MiB), defeats GPU attacks
- **Time-cost**: ~100ms on modern hardware
- **Parallelism**: Single-threaded (parallelism=1) for KDFs
- **Version**: v1.3 (latest RFC 9106)

**Security:**
- Resistant to GPU/ASIC attacks
- Memory requirement increases attacker hardware costs
- Time-space tradeoff makes parallelization expensive
- Current best practice for password hashing

**Use When:**
- Security against GPU attacks is important
- Master password could be weak
- Defending against well-resourced attackers

### 3.2 Salt Generation

Both KDFs require a random salt:

```rust
// Generate 256-bit (32-byte) random salt
let salt = random_bytes(32);

// Store with key derivation parameters
master_key_info.salt = salt;
master_key_info.algorithm = "argon2id" | "pbkdf2";
```

**Salt Properties:**
- **Size**: 32 bytes minimum
- **Randomness**: Cryptographically secure RNG
- **Uniqueness**: Different salt for each user/deployment
- **Storage**: Publicly stored (not sensitive)

**Rationale:** Salt prevents rainbow table attacks and ensures identical passwords produce different keys across deployments.

### 3.3 Computation Time

Both configured for ~100ms on 2024 hardware:

| KDF | Time | CPU | GPU | ASIC |
|-----|------|-----|-----|------|
| PBKDF2 | 100ms | ~1x | 100x+ faster | 1000x+ faster |
| Argon2id | 100ms | ~1x | 10x+ harder | 100x+ harder |

**Implication:** With Argon2id:
- Brute-forcing 1000 passwords = 100 seconds on CPU
- Same attack on GPU is still expensive (19 MiB per attempt)
- Attacker would need 1000 × 19 MiB = 19 GB GPU memory

---

## 4. Database Schema

### 4.1 Master Keys Table

Stores key derivation parameters and metadata:

```sql
CREATE TABLE master_keys (
    id TEXT PRIMARY KEY,          -- UUID format
    algorithm TEXT NOT NULL,      -- 'pbkdf2' or 'argon2id'
    salt BLOB NOT NULL,           -- 32 bytes random
    iterations INTEGER,           -- For PBKDF2 only
    memory_cost INTEGER,          -- For Argon2id only (19456 = 19 MiB)
    parallelism INTEGER,          -- For Argon2id only
    key_length INTEGER NOT NULL,  -- Always 32 for AES-256
    password_hash BLOB NOT NULL,  -- Scrypt(password) for verification
    created_at INTEGER NOT NULL,  -- Unix timestamp (seconds)
    last_rotated_at INTEGER,      -- Unix timestamp (seconds)
    rotation_interval_days INTEGER DEFAULT 90,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    version INTEGER NOT NULL DEFAULT 1
);
```

**Indices:**
- PRIMARY KEY: id
- Algorithm: Used during initialization

**Design Rationale:**
- Stores all parameters needed to re-derive key
- Password hash allows verification without storing password
- Rotation tracking enables master key rotation
- Algorithm versioning allows future changes

### 4.2 Secrets Table

Stores metadata only (never plaintext values):

```sql
CREATE TABLE secrets (
    id TEXT PRIMARY KEY,              -- UUID format
    name TEXT NOT NULL UNIQUE,        -- Human name, unique
    secret_type TEXT NOT NULL,        -- api_key, oauth_token, etc
    description TEXT,                 -- Purpose of secret
    service TEXT,                     -- Associated service name
    current_version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,      -- Unix timestamp
    updated_at INTEGER NOT NULL,      -- Unix timestamp
    last_accessed_at INTEGER,         -- Unix timestamp
    expires_at INTEGER,               -- Unix timestamp (optional)
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_by TEXT NOT NULL,         -- User who created
    updated_by TEXT                   -- Last user to modify
);

CREATE UNIQUE INDEX idx_secrets_name ON secrets(name);
```

**Indices:**
- UNIQUE (name): Prevent duplicate secret names
- Standard queries: by id (primary key)

### 4.3 Secret Values Table

Stores encrypted data and encryption parameters:

```sql
CREATE TABLE secret_values (
    id TEXT PRIMARY KEY,              -- UUID for this version
    secret_id TEXT NOT NULL,          -- FK to secrets
    version INTEGER NOT NULL,         -- Version number (1, 2, 3, ...)
    ciphertext BLOB NOT NULL,         -- AES-256-GCM encrypted data
    nonce BLOB NOT NULL,              -- 12-byte random nonce
    tag BLOB NOT NULL,                -- 16-byte GCM auth tag
    encryption_version INTEGER NOT NULL DEFAULT 1,  -- Algorithm version
    created_at INTEGER NOT NULL,      -- When version created
    rotated_at INTEGER,               -- When rotated away
    is_valid BOOLEAN NOT NULL DEFAULT 1,  -- Still usable?

    UNIQUE(secret_id, version),
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
);

CREATE INDEX idx_secret_values_secret_id ON secret_values(secret_id);
```

**Design Rationale:**
- Separate table enables version history
- Stores both ciphertext and nonce (both needed for decryption)
- Nonce is public (stored unencrypted)
- Each version is immutable
- UNIQUE constraint prevents duplicate versions

### 4.4 Access Control Table

Fine-grained permission management:

```sql
CREATE TABLE access_control (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,       -- User/role/service ID
    principal_type TEXT NOT NULL,     -- 'user', 'role', 'service'
    permission_level INTEGER NOT NULL, -- 0-5 (None to Admin)
    granted_at INTEGER NOT NULL,
    granted_by TEXT NOT NULL,
    expires_at INTEGER,               -- Optional expiration
    reason TEXT,

    UNIQUE(secret_id, principal_id, principal_type),
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
);

CREATE INDEX idx_acl_secret_id ON access_control(secret_id);
CREATE INDEX idx_acl_principal_id ON access_control(principal_id);
```

**Permission Levels (0-5):**
```
0 = None          - No access
1 = ViewMetadata  - Can see metadata only
2 = Read          - Can decrypt and read value
3 = Update        - Can change/rotate value
4 = Delete        - Can delete secret
5 = Admin         - Full control
```

**Access Check Algorithm:**
```
function canAccess(secret_id, principal_id, required_level):
    entry = access_control WHERE secret_id AND principal_id

    if entry NOT FOUND:
        return false

    if entry.expires_at IS NOT NULL AND entry.expires_at < NOW():
        return false  -- permission expired

    return entry.permission_level >= required_level
```

### 4.5 Audit Log Table

Complete audit trail of all operations:

```sql
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,          -- Which secret
    principal_id TEXT NOT NULL,       -- Who accessed it
    action TEXT NOT NULL,             -- create/read/update/delete/rotate
    success BOOLEAN NOT NULL,         -- Did it work?
    error TEXT,                       -- Error message if failed
    timestamp INTEGER NOT NULL,       -- Unix timestamp (seconds)
    source TEXT,                      -- IP address or identifier
    context TEXT,                     -- JSON context

    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
);

CREATE INDEX idx_audit_secret_id ON audit_logs(secret_id);
CREATE INDEX idx_audit_principal_id ON audit_logs(principal_id);
CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp);
CREATE INDEX idx_audit_action ON audit_logs(action);
```

**Action Types:**
- `create`: Secret created
- `read`: Secret value accessed
- `update`: Secret value modified
- `delete`: Secret deleted
- `rotate`: Secret version rotated
- `modify_metadata`: Description/tags changed
- `change_permissions`: ACL modified
- `export`: Secret exported
- `access_denied`: Access attempt blocked

**Audit Entry Structure:**
```json
{
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "secret_id": "550e8400-e29b-41d4-a716-446655440001",
    "principal_id": "user@example.com",
    "action": "read",
    "success": true,
    "error": null,
    "timestamp": 1700000000,
    "source": "192.168.1.100",
    "context": {
        "user_agent": "curl/7.68.0",
        "reason": "CI/CD pipeline",
        "service": "github-actions"
    }
}
```

---

## 5. Access Control Model

### 5.1 Permission Model

Four-level hierarchy:

```
Admin (5)
  ├─ Delete (4)
  ├─ Update (3)
  ├─ Read (2)
  ├─ ViewMetadata (1)
  └─ None (0)
```

**Permission Semantics:**

| Level | Capability | Use Case |
|-------|-----------|----------|
| ViewMetadata | See name, type, description | Secret discovery |
| Read | Decrypt and use value | Service authentication |
| Update | Change secret to new value | Credential rotation |
| Delete | Permanently remove secret | Cleanup |
| Admin | Modify ACLs, export | Secret management |

### 5.2 Principal Types

Three types of access principals:

```rust
pub enum PrincipalType {
    User,     // Individual person (e.g., "alice@example.com")
    Role,     // Group of users (e.g., "backend-team", "admins")
    Service,  // Service account (e.g., "github-actions", "lambda-processor")
}
```

### 5.3 Permission Checks

Check permissions before any operation:

```
function getSecret(secret_id, principal_id, operation):
    // Check permission
    level = getPermissionLevel(secret_id, principal_id)
    required_level = getRequiredLevel(operation)

    if level < required_level:
        logAudit(secret_id, principal_id, "access_denied", false)
        throw AccessDenied

    // Check expiration
    if isExpired(secret_id, principal_id):
        logAudit(secret_id, principal_id, "access_denied", false)
        throw AccessDenied

    // Perform operation
    secret = decrypt(secret_id)
    logAudit(secret_id, principal_id, operation, true)
    return secret
```

---

## 6. Secret Rotation

### 6.1 Rotation Mechanism

Secrets can be rotated to new values while maintaining history:

```
Old Value (Version 1)    Version 1 created at T0
    ↓ [Rotate]
New Value (Version 2)    Version 2 created at T1, V1 marked rotated_at=T1
    ↓ [Rotate]
Newer Value (Version 3)  Version 3 created at T2, V2 marked rotated_at=T2
```

**Database State:**
```sql
-- secret_values for secret_id="github-token"
SELECT version, is_valid, rotated_at FROM secret_values;

version | is_valid | rotated_at
--------|----------|----------
  1     | 1        | T1         -- Old value, rotated away but valid
  2     | 1        | T2         -- Older value, still valid for emergency
  3     | 1        | NULL       -- Current value (no rotation time)
```

### 6.2 Rotation Policies

Optional automatic rotation policies:

```sql
CREATE TABLE rotation_policies (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL UNIQUE,
    rotation_interval_days INTEGER NOT NULL,  -- Rotate every N days
    rotation_strategy TEXT,                    -- 'automatic', 'manual', 'on_access'
    last_rotated_at INTEGER,
    next_rotation_at INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT 1
);
```

**Rotation Strategies:**
1. **Automatic**: System rotates on schedule (e.g., every 90 days)
2. **Manual**: Only rotate on explicit request
3. **On Access**: Rotate immediately on first access each day

### 6.3 Emergency Recovery

Keep last N versions valid for recovery:

```
Current version: 3
Keep valid: versions 1, 2, 3
Delete after: 30+ days old
```

This allows recovery if:
- New secret doesn't work
- Service reverted to old key
- Emergency fallback needed

---

## 7. Audit & Compliance

### 7.1 Comprehensive Logging

Every operation is logged:

```
Operation Timeline:

T0: Alice creates secret "db-password"
    audit_logs: {action: "create", principal_id: "alice@acme.com", success: true}

T1: Alice grants Read to Backend Team
    audit_logs: {action: "change_permissions", principal_id: "alice@acme.com", success: true}

T2: Backend service reads secret
    audit_logs: {action: "read", principal_id: "backend-service", success: true}

T3: Attacker tries to read secret (blocked)
    audit_logs: {action: "read", principal_id: "attacker@evil.com", success: false, error: "access_denied"}

T4: Alice rotates secret to new value
    audit_logs: {action: "rotate", principal_id: "alice@acme.com", success: true}
```

### 7.2 Compliance Requirements Met

| Requirement | Implementation |
|-------------|-----------------|
| Encryption at rest | AES-256-GCM |
| Key derivation | PBKDF2 or Argon2id |
| Access control | Role-based with levels |
| Audit logging | Immutable audit trail |
| Authentication | Master password + ACL |
| Integrity | GCM authentication tags |
| Key rotation | Master key rotation support |

---

## 8. Error Handling

### 8.1 Error Types (StateStoreError Extensions)

```rust
pub enum StateStoreError {
    // Existing errors
    DatabaseError(String),
    NotFound(String),

    // New secrets-specific errors
    EncryptionError(String),      // AES-GCM failed
    DecryptionFailed,             // Tag verification failed
    InvalidSecret(String),        // Corrupted or invalid
    AccessDenied(String),         // Permission check failed
    AuthenticationFailed(String), // Password wrong
    MasterKeyNotInitialized,      // No master key set up
    InvalidPassword(String),      // Weak password
    RotationFailed(String),       // Rotation process failed
    ExpiredSecret(String),        // Expiration date passed
}
```

### 8.2 Error Recovery

Some operations can be retried:

```
RETRY: Database errors, network timeouts
CANNOT RETRY: Authentication failures, permission denied
```

---

## 9. Testing Checklist

### 9.1 Cryptographic Tests

- [ ] AES-256-GCM encryption/decryption roundtrip
- [ ] Nonce uniqueness verification
- [ ] Authentication tag verification with wrong password
- [ ] Salt generation randomness
- [ ] PBKDF2 key derivation determinism
- [ ] Argon2id key derivation determinism

### 9.2 Database Tests

- [ ] Schema creation and migrations
- [ ] Foreign key constraints enforced
- [ ] Unique constraints (secret names, versions)
- [ ] Index performance
- [ ] Audit logging completeness

### 9.3 Access Control Tests

- [ ] Permission checking works correctly
- [ ] Expiration enforcement
- [ ] Different principal types
- [ ] Cascading deletes

### 9.4 Security Tests

- [ ] No plaintext secrets in database
- [ ] No ciphertext reuse
- [ ] Audit trail immutability
- [ ] Rate limiting on failures
- [ ] Memory zeroization

---

## 10. Future Extensibility

### 10.1 Algorithm Migration

Support future encryption algorithms:

```sql
-- encryption_version in secret_values allows:
-- Version 1: AES-256-GCM (current)
-- Version 2: ChaCha20-Poly1305 (future)
-- Version 3: Post-quantum algorithm (future)
```

### 10.2 KDF Migration

Support future key derivation:

```sql
-- kdf_version in master_keys enables:
-- Version 1: PBKDF2 / Argon2id (current)
-- Version 2: Newer algorithm (future)
-- Allows gradual migration
```

### 10.3 Hardware Security Module (HSM)

Future: Store master key in HSM instead of deriving from password

---

## Conclusion

This design provides:
- **Confidentiality**: AES-256-GCM encryption
- **Authenticity**: GCM authentication tags
- **Integrity**: Constraint checks and audit logs
- **Access Control**: Fine-grained permissions
- **Auditability**: Complete operation logs
- **Extensibility**: Algorithm versioning for future changes
