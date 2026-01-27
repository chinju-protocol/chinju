# Error Codes - エラーコード標準

## なぜ必要か（背景）

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  エラーがバラバラだと...                                     │
│                                                             │
│  • デバッグが困難（何のエラー？）                            │
│  • 自動リトライ判定不可（一時的？恒久的？）                  │
│  • ログ分析不可（エラー種別でフィルタできない）              │
│  • 国際化困難（エラーメッセージが固定）                      │
│                                                             │
│  各トレイトが独自のError型を持つと、統一的な処理ができない  │
└─────────────────────────────────────────────────────────────┘
```

---

## 1. エラーコード体系

### 1.1 形式

```
CHJ-<category>-<number>

例:
  CHJ-HW-0001  （ハードウェアエラー）
  CHJ-VAL-0042 （バリデーションエラー）
  CHJ-SEC-0100 （セキュリティエラー）
```

### 1.2 カテゴリ

| カテゴリ | コード | 説明 |
|---------|--------|------|
| Hardware | `HW` | ハードウェア層のエラー |
| Validation | `VAL` | 入力検証エラー |
| Security | `SEC` | セキュリティ関連エラー |
| Protocol | `PROT` | プロトコルエラー |
| Version | `VER` | バージョン互換性エラー |
| Threshold | `THR` | 閾値違反 |
| Audit | `AUD` | 監査エラー |
| Config | `CFG` | 設定エラー |
| Internal | `INT` | 内部エラー |

---

## 2. エラーコード一覧

### 2.1 Hardware (HW)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-HW-0001 | HsmUnavailable | HSMが利用不可 | はい（一時的） |
| CHJ-HW-0002 | HsmKeyNotFound | 指定鍵スロットが存在しない | いいえ |
| CHJ-HW-0003 | HsmKeyErased | 鍵が消去済み | いいえ |
| CHJ-HW-0004 | TeeAttestationFailed | TEEアテステーション失敗 | はい |
| CHJ-HW-0005 | QrngEntropyLow | QRNG エントロピー不足 | はい |
| CHJ-HW-0006 | OtpReadError | OTP読み取りエラー | はい |
| CHJ-HW-0007 | OtpAlreadyBurned | OTP既に書き込み済み | いいえ |
| CHJ-HW-0008 | TamperDetected | タンパー検出 | いいえ（即停止） |
| CHJ-HW-0009 | DiodeBufferOverflow | データダイオードバッファ溢れ | はい |
| CHJ-HW-0010 | ClockDesync | GPS時刻同期外れ | はい |

### 2.2 Validation (VAL)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-VAL-0001 | MissingRequiredField | 必須フィールドが欠落 | いいえ（修正要） |
| CHJ-VAL-0002 | InvalidFieldType | フィールド型が不正 | いいえ |
| CHJ-VAL-0003 | OutOfRange | 値が許容範囲外 | いいえ |
| CHJ-VAL-0004 | InvalidFormat | 形式が不正 | いいえ |
| CHJ-VAL-0005 | ChecksumMismatch | チェックサム不一致 | いいえ |

### 2.3 Security (SEC)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-SEC-0001 | SignatureInvalid | 署名検証失敗 | いいえ |
| CHJ-SEC-0002 | InsufficientSigners | 署名者数が閾値未満 | いいえ |
| CHJ-SEC-0003 | CertificateExpired | 証明書期限切れ | いいえ（更新要） |
| CHJ-SEC-0004 | CertificateRevoked | 証明書失効済み | いいえ |
| CHJ-SEC-0005 | UnauthorizedAccess | 権限なしアクセス | いいえ |
| CHJ-SEC-0006 | IntegrityViolation | 整合性違反（改ざん検知） | いいえ（即停止） |
| CHJ-SEC-0007 | ReplayDetected | リプレイ攻撃検知 | いいえ |

### 2.4 Protocol (PROT)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-PROT-0001 | MissingSchemaVersion | schema_version 欠落 | いいえ |
| CHJ-PROT-0002 | MissingDefinitionVersion | definition_version 欠落 | いいえ |
| CHJ-PROT-0003 | UnsupportedPayloadType | 未対応ペイロード種別 | いいえ |
| CHJ-PROT-0004 | MalformedPayload | ペイロード構造不正 | いいえ |

### 2.5 Version (VER)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-VER-0001 | UnsupportedVersion | 未対応バージョン | いいえ（アップグレード要） |
| CHJ-VER-0002 | DeprecatedVersion | 非推奨バージョン | はい（警告） |
| CHJ-VER-0003 | VersionMismatch | バージョン不一致 | いいえ |
| CHJ-VER-0004 | MigrationRequired | マイグレーション必要 | いいえ（処理要） |

### 2.6 Threshold (THR)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-THR-0001 | DStarExceeded | D* が閾値超過 | いいえ（停止） |
| CHJ-THR-0002 | NEffInsufficient | N_eff が閾値未満 | いいえ（停止） |
| CHJ-THR-0003 | SrciAnomalous | SRCI が異常範囲 | はい（警告） |
| CHJ-THR-0004 | TokensDepleted | トークン枯渇 | いいえ（停止） |
| CHJ-THR-0005 | CapabilityInsufficient | 人間の制御能力不足 | いいえ（制限） |

### 2.7 Audit (AUD)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-AUD-0001 | AuditLogFull | 監査ログ容量超過 | はい（ローテーション後） |
| CHJ-AUD-0002 | AuditLogCorrupted | 監査ログ破損 | いいえ（即停止） |
| CHJ-AUD-0003 | AuditWriteFailed | 監査ログ書き込み失敗 | はい |

### 2.8 Config (CFG)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-CFG-0001 | ConfigNotFound | 設定ファイルなし | いいえ（設定要） |
| CHJ-CFG-0002 | ConfigParseError | 設定パースエラー | いいえ |
| CHJ-CFG-0003 | ConfigValidationFailed | 設定検証失敗 | いいえ |

### 2.9 Internal (INT)

| コード | 名前 | 説明 | リトライ可能 |
|--------|------|------|-------------|
| CHJ-INT-0001 | UnexpectedState | 予期しない状態 | いいえ（バグ） |
| CHJ-INT-0002 | Timeout | タイムアウト | はい |
| CHJ-INT-0003 | ResourceExhausted | リソース枯渇 | はい |

---

## 3. 共通エラー型

### 3.1 Rust実装

```rust
use std::fmt;

/// CHINJUエラーコード
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChinjuErrorCode {
    /// カテゴリ（例: "HW", "SEC"）
    pub category: String,
    /// 番号（例: 1, 42）
    pub number: u16,
}

impl ChinjuErrorCode {
    pub fn new(category: &str, number: u16) -> Self {
        Self {
            category: category.to_string(),
            number,
        }
    }

    /// フォーマット済みコードを取得
    pub fn code(&self) -> String {
        format!("CHJ-{}-{:04}", self.category, self.number)
    }
}

impl fmt::Display for ChinjuErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// 共通エラー型
#[derive(Debug)]
pub struct ChinjuError {
    /// エラーコード
    pub code: ChinjuErrorCode,
    /// 人間可読メッセージ（デバッグ用）
    pub message: String,
    /// 詳細情報（オプション）
    pub details: Option<serde_json::Value>,
    /// リトライ可能か
    pub retryable: bool,
    /// 元のエラー（ラップ時）
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ChinjuError {
    pub fn new(code: ChinjuErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            retryable: false,
            source: None,
        }
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::error::Error for ChinjuError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl fmt::Display for ChinjuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
```

### 3.2 エラーコード定数

```rust
/// 定義済みエラーコード
pub mod codes {
    use super::ChinjuErrorCode;

    // Hardware
    pub const HSM_UNAVAILABLE: ChinjuErrorCode = ChinjuErrorCode { category: "HW".to_string(), number: 1 };
    pub const TAMPER_DETECTED: ChinjuErrorCode = ChinjuErrorCode { category: "HW".to_string(), number: 8 };

    // Security
    pub const SIGNATURE_INVALID: ChinjuErrorCode = ChinjuErrorCode { category: "SEC".to_string(), number: 1 };
    pub const INTEGRITY_VIOLATION: ChinjuErrorCode = ChinjuErrorCode { category: "SEC".to_string(), number: 6 };

    // Threshold
    pub const DSTAR_EXCEEDED: ChinjuErrorCode = ChinjuErrorCode { category: "THR".to_string(), number: 1 };
    pub const NEFF_INSUFFICIENT: ChinjuErrorCode = ChinjuErrorCode { category: "THR".to_string(), number: 2 };
    pub const TOKENS_DEPLETED: ChinjuErrorCode = ChinjuErrorCode { category: "THR".to_string(), number: 4 };
}
```

---

## 4. エラーレスポンス

### 4.1 JSON形式

```json
{
  "error": {
    "code": "CHJ-SEC-0001",
    "message": "Signature verification failed",
    "retryable": false,
    "details": {
      "expected_signers": 3,
      "actual_signers": 1,
      "missing_signers": ["key:abc123", "key:def456"]
    },
    "timestamp": "2024-01-15T10:30:00Z",
    "trace_id": "req-12345"
  }
}
```

### 4.2 ログ形式

```
[ERROR] CHJ-THR-0001 D* exceeded threshold | d_star=0.85 threshold=0.8 | trace_id=req-12345
[WARN]  CHJ-VER-0002 Deprecated version | version=metrics_v1 deprecated_since=2024-01-01
[INFO]  CHJ-HW-0005 QRNG entropy recovered | entropy=0.98
```

---

## 5. 即時停止エラー

以下のエラーは**即座にシステムを停止**させる：

| コード | 理由 |
|--------|------|
| CHJ-HW-0008 | タンパー検出（物理攻撃） |
| CHJ-SEC-0006 | 整合性違反（改ざん検知） |
| CHJ-AUD-0002 | 監査ログ破損（追跡不能） |
| CHJ-THR-0001 | D* 超過（AI支配の兆候） |
| CHJ-THR-0002 | N_eff 不足（多様性喪失） |
| CHJ-THR-0004 | トークン枯渇（代謝停止） |

---

## 6. トレイトのエラー型統一

### 6.1 既存トレイトの変更

```rust
// Before: 各トレイトが独自のエラー型
pub trait HardwareSecurityModule {
    type Error: std::error::Error;  // バラバラ
    fn sign(&self, ...) -> Result<Signature, Self::Error>;
}

// After: 共通エラー型を使用
pub trait HardwareSecurityModule {
    fn sign(&self, ...) -> Result<Signature, ChinjuError>;
}
```

### 6.2 エラー変換

```rust
impl From<YubiHsmError> for ChinjuError {
    fn from(err: YubiHsmError) -> Self {
        match err {
            YubiHsmError::DeviceNotFound => ChinjuError::new(
                codes::HSM_UNAVAILABLE,
                "YubiHSM device not found"
            ).retryable(),
            YubiHsmError::KeyNotFound(slot) => ChinjuError::new(
                codes::HSM_KEY_NOT_FOUND,
                format!("Key not found in slot {}", slot)
            ),
            // ...
        }
    }
}
```

---

## 参照文書

- `chinju/spec/PROTOCOL_PROTECTION.md` - プロトコル保護
- `chinju/spec/VERSIONING.md` - バージョン管理
- `chinju/traits/hardware_abstraction.md` - 物理層トレイト
