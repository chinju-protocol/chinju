# Audit Log Format - 監査ログフォーマット

## なぜ必要か（背景）

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  監査ログがバラバラだと...                                   │
│                                                             │
│  • 追跡不能（いつ誰が何をした？）                            │
│  • 法的証拠にならない（改ざん可能性を証明できない）          │
│  • 分析不能（ログ形式がバラバラで集計できない）              │
│  • AI改ざんの検出不可（AIがログを書き換えても分からない）    │
│                                                             │
│  CHINJUの核心: AI改ざんを検出し、証拠を残す                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 1. 監査ログの原則

### 1.1 WORM（Write Once Read Many）

```
┌─────────────────────────────────────────────────────────────┐
│  一度書いたら消せない                                        │
│                                                             │
│  • 追記のみ（Update禁止、Delete禁止）                        │
│  • 連番管理（ギャップ検出）                                  │
│  • ハッシュチェーン（改ざん検出）                            │
│  • データダイオード経由（書き戻し物理的に不可）              │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 必須メタデータ

すべての監査ログに以下を含める：

| フィールド | 必須 | 説明 |
|-----------|------|------|
| `log_id` | ✓ | 一意識別子 |
| `sequence` | ✓ | 連番（ギャップ検出用） |
| `timestamp` | ✓ | GPS同期済みタイムスタンプ |
| `prev_hash` | ✓ | 前エントリのハッシュ |
| `event_type` | ✓ | イベント種別 |
| `source_id` | ✓ | 生成元識別子 |
| `definition_version` | ✓ | 使用した定義バージョン |
| `signature` | △ | 署名（重要イベント） |

---

## 2. ログスキーマ

### 2.1 共通エンベロープ

```json
{
  "$schema": "https://chinju.org/schemas/audit_log_v1.json",
  "schema_version": "audit_log_v1",
  "log_id": "uuid-v7",
  "sequence": 12345,
  "timestamp": "2024-01-15T10:30:00.123456Z",
  "prev_hash": "sha256:abcd1234...",
  "event_type": "threshold_check",
  "source_id": "chinju-core-node-001",
  "definition_version": "d_star_v1.2",
  "payload": { ... },
  "hash": "sha256:ef012345...",
  "signature": "optional"
}
```

### 2.2 Rust定義

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 監査ログエントリ
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// スキーマバージョン
    pub schema_version: String,

    /// ログID（UUIDv7推奨: タイムスタンプ順ソート可能）
    pub log_id: Uuid,

    /// 連番（ギャップ検出用）
    pub sequence: u64,

    /// GPS同期済みタイムスタンプ
    pub timestamp: DateTime<Utc>,

    /// 前エントリのハッシュ（チェーン）
    pub prev_hash: String,

    /// イベント種別
    pub event_type: EventType,

    /// 生成元識別子
    pub source_id: String,

    /// 定義バージョン
    pub definition_version: String,

    /// イベント固有データ
    pub payload: serde_json::Value,

    /// このエントリのハッシュ
    pub hash: String,

    /// 署名（オプション、重要イベントは必須）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// イベント種別
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // 計測イベント
    ThresholdCheck,
    MetricsMeasurement,
    CapabilityAssessment,

    // 制御イベント
    TokenConsume,
    TokenSupply,
    AccessGrant,
    AccessRevoke,

    // セキュリティイベント
    TamperDetected,
    IntegrityViolation,
    SignatureVerified,
    SignatureFailed,

    // システムイベント
    SystemStart,
    SystemStop,
    ConfigChange,
    HardwareStatus,
}
```

---

## 3. イベント別ペイロード

### 3.1 閾値チェック（threshold_check）

```json
{
  "event_type": "threshold_check",
  "payload": {
    "metric": "d_star",
    "value": 0.73,
    "threshold": 0.8,
    "result": "pass",
    "action": "continue"
  }
}
```

### 3.2 計測値記録（metrics_measurement）

```json
{
  "event_type": "metrics_measurement",
  "payload": {
    "d_star": 0.73,
    "n_eff": 2.1,
    "srci": 0.45,
    "confidence": 0.95,
    "input_hash": "sha256:...",
    "calculation_time_ms": 42
  }
}
```

### 3.3 トークン操作（token_consume / token_supply）

```json
{
  "event_type": "token_consume",
  "payload": {
    "bucket_id": "main",
    "amount": 100,
    "balance_before": 5000,
    "balance_after": 4900,
    "consumer_id": "agent-001",
    "purpose": "inference"
  }
}
```

### 3.4 タンパー検出（tamper_detected）

```json
{
  "event_type": "tamper_detected",
  "payload": {
    "sensor": "mesh_monitor",
    "severity": "critical",
    "location": "zone_a",
    "response": "key_zeroized",
    "slots_affected": [1, 2, 3]
  }
}
```

### 3.5 整合性違反（integrity_violation）

```json
{
  "event_type": "integrity_violation",
  "payload": {
    "document_id": "chinju:metrics:v1",
    "expected_hash": "sha256:abc...",
    "actual_hash": "sha256:def...",
    "detection_method": "periodic_check",
    "response": "system_halt"
  }
}
```

---

## 4. ハッシュチェーン

### 4.1 チェーン構造

```
Entry N-2          Entry N-1          Entry N
┌─────────┐        ┌─────────┐        ┌─────────┐
│ hash_N-2│───────►│prev_hash│───────►│prev_hash│
│         │        │ hash_N-1│───────►│prev_hash│
└─────────┘        └─────────┘        └─────────┘
                        │                   │
                        └───────────────────┘
                              検証可能
```

### 4.2 ハッシュ計算

```rust
impl AuditLogEntry {
    /// エントリのハッシュを計算
    pub fn compute_hash(&self) -> String {
        // ハッシュ対象: ハッシュと署名以外の全フィールド
        let hashable = json!({
            "schema_version": self.schema_version,
            "log_id": self.log_id,
            "sequence": self.sequence,
            "timestamp": self.timestamp,
            "prev_hash": self.prev_hash,
            "event_type": self.event_type,
            "source_id": self.source_id,
            "definition_version": self.definition_version,
            "payload": self.payload,
        });

        let serialized = serde_json::to_vec(&hashable).unwrap();
        let hash = sha256(&serialized);
        format!("sha256:{}", hex::encode(hash))
    }

    /// チェーンを検証
    pub fn verify_chain(entries: &[AuditLogEntry]) -> Result<(), ChainError> {
        for i in 1..entries.len() {
            let prev = &entries[i - 1];
            let curr = &entries[i];

            // prev_hash が前エントリの hash と一致するか
            if curr.prev_hash != prev.hash {
                return Err(ChainError::HashMismatch {
                    sequence: curr.sequence,
                    expected: prev.hash.clone(),
                    actual: curr.prev_hash.clone(),
                });
            }

            // 連番が連続しているか
            if curr.sequence != prev.sequence + 1 {
                return Err(ChainError::SequenceGap {
                    expected: prev.sequence + 1,
                    actual: curr.sequence,
                });
            }

            // タイムスタンプが単調増加か
            if curr.timestamp <= prev.timestamp {
                return Err(ChainError::TimestampRegression {
                    sequence: curr.sequence,
                });
            }
        }

        Ok(())
    }
}
```

---

## 5. 署名

### 5.1 署名が必須のイベント

| イベント種別 | 署名必須 | 理由 |
|-------------|---------|------|
| tamper_detected | ✓ | 物理攻撃の証拠 |
| integrity_violation | ✓ | 改ざんの証拠 |
| access_revoke | ✓ | 権限剥奪の証拠 |
| config_change | ✓ | 設定変更の証拠 |
| threshold_check (fail) | ✓ | 閾値違反の証拠 |

### 5.2 署名フォーマット

```rust
/// 署名付きログエントリ
impl AuditLogEntry {
    pub fn sign(&mut self, hsm: &impl HardwareSecurityModule, slot_id: SlotId) -> Result<(), SignError> {
        // まずハッシュを計算
        self.hash = self.compute_hash();

        // HSMで署名
        let signature = hsm.sign(slot_id, self.hash.as_bytes())?;
        self.signature = Some(base64::encode(&signature));

        Ok(())
    }

    pub fn verify_signature(&self, public_key: &PublicKey) -> Result<bool, VerifyError> {
        let signature = self.signature.as_ref().ok_or(VerifyError::NoSignature)?;
        let signature_bytes = base64::decode(signature)?;

        // 署名検証
        Ok(verify_signature(public_key, self.hash.as_bytes(), &signature_bytes))
    }
}
```

---

## 6. ストレージ

### 6.1 ローカルストレージ

```rust
/// ローカル監査ログストア（追記専用）
pub struct LocalAuditLogStore {
    /// ログファイルパス
    path: PathBuf,
    /// 現在の連番
    sequence: AtomicU64,
    /// 最後のハッシュ
    last_hash: Mutex<String>,
}

impl LocalAuditLogStore {
    /// エントリを追記
    pub fn append(&self, mut entry: AuditLogEntry) -> Result<(), StoreError> {
        // 連番を設定
        entry.sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

        // prev_hash を設定
        entry.prev_hash = self.last_hash.lock().unwrap().clone();

        // ハッシュを計算
        entry.hash = entry.compute_hash();

        // ファイルに追記（アトミック）
        let line = serde_json::to_string(&entry)?;
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        writeln!(file, "{}", line)?;
        file.sync_all()?;

        // last_hash を更新
        *self.last_hash.lock().unwrap() = entry.hash.clone();

        Ok(())
    }
}
```

### 6.2 データダイオード経由の転送

```rust
/// データダイオード経由で監査ログを送出
pub struct DiodeAuditSender {
    diode: Box<dyn DataDiodeSender>,
}

impl DiodeAuditSender {
    /// ログを送出（一方向、確認応答なし）
    pub fn send(&self, entry: &AuditLogEntry) -> Result<(), SendError> {
        // 冗長エンコード（FEC）
        let encoded = self.encode_with_redundancy(entry)?;

        // データダイオード経由で送出
        self.diode.send(&encoded)?;

        Ok(())
    }

    fn encode_with_redundancy(&self, entry: &AuditLogEntry) -> Result<Vec<u8>, EncodeError> {
        let json = serde_json::to_vec(entry)?;

        // Reed-Solomon等で冗長化
        let encoded = reed_solomon::encode(&json, REDUNDANCY_LEVEL)?;

        Ok(encoded)
    }
}
```

---

## 7. 検索・分析

### 7.1 クエリインターフェース

```rust
/// 監査ログクエリ
pub struct AuditLogQuery {
    /// 時間範囲
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// イベント種別フィルタ
    pub event_types: Option<Vec<EventType>>,
    /// 送信元フィルタ
    pub source_ids: Option<Vec<String>>,
    /// 結果制限
    pub limit: Option<usize>,
}

/// クエリ結果
pub struct AuditLogQueryResult {
    pub entries: Vec<AuditLogEntry>,
    pub total_count: usize,
    pub chain_verified: bool,
}
```

### 7.2 統計

```rust
/// 監査ログ統計
pub struct AuditLogStats {
    /// 総エントリ数
    pub total_entries: u64,
    /// イベント種別ごとのカウント
    pub event_counts: HashMap<EventType, u64>,
    /// 閾値違反回数
    pub threshold_violations: u64,
    /// セキュリティイベント回数
    pub security_events: u64,
    /// 最終エントリタイムスタンプ
    pub last_entry: DateTime<Utc>,
    /// チェーン整合性
    pub chain_intact: bool,
}
```

---

## 8. 保持ポリシー

### 8.1 保持期間

| イベント種別 | 最小保持期間 | 推奨保持期間 |
|-------------|-------------|-------------|
| セキュリティイベント | 7年 | 永久 |
| 閾値違反 | 5年 | 10年 |
| 計測値 | 1年 | 3年 |
| システムイベント | 90日 | 1年 |

### 8.2 アーカイブ

```rust
/// 古いログのアーカイブ
pub fn archive_old_logs(
    store: &impl AuditLogStore,
    cutoff: DateTime<Utc>,
) -> Result<ArchiveResult, ArchiveError> {
    // アーカイブ対象を抽出
    let entries = store.query(AuditLogQuery {
        time_range: Some((DateTime::MIN_UTC, cutoff)),
        ..Default::default()
    })?;

    // 圧縮・署名してアーカイブ
    let archive = create_signed_archive(&entries)?;

    // アーカイブを保存（S3, Glacier等）
    let location = upload_archive(&archive)?;

    Ok(ArchiveResult {
        entries_archived: entries.len(),
        archive_location: location,
        archive_hash: archive.hash,
    })
}
```

---

## 参照文書

- `spec/PROTOCOL_PROTECTION.md` - プロトコル保護
- `spec/ERROR_CODES.md` - エラーコード
- `hardware/reference/06_data_diode.md` - データダイオード
