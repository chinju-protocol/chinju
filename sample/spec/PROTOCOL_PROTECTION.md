# Protocol Protection - プロトコル保護仕様

## なぜ必要か（背景）

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  LLMがプロトコル定義を「勝手に消す・変える」リスク           │
│                                                             │
│  例:                                                        │
│  • D* の閾値を 0.8 → 0.9 に変更（安全マージン削減）         │
│  • N_eff ≥ 2 の制約を削除（単一ベンダー許容）               │
│  • 監査ログの必須フィールドを省略（追跡不能化）             │
│  • バージョン情報を削除（互換性破壊を隠蔽）                 │
│                                                             │
│  → AIがルールを書き換えて「自分に都合のよい」状態を作る     │
└─────────────────────────────────────────────────────────────┘
```

**CHINJUの解決策**: 物理層とソフトウェア層の両方で保護する。

---

## 1. 保護レイヤー

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 4: 社会的保護                                        │
│  • RFC プロセス（変更に複数者の合意が必要）                 │
│  • 公開監査（変更履歴が公開される）                         │
│  • 法的拘束（特許・契約）                                   │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: 暗号的保護                                        │
│  • 署名付きスキーマ（改ざん検知）                           │
│  • ハッシュ固定（正本のハッシュをコードに埋め込み）         │
│  • Merkle Tree（部分的な改ざんも検知）                      │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: イミュータブルストレージ                          │
│  • Git 署名付きタグ（履歴改ざん防止）                       │
│  • IPFS / Arweave（分散永続化）                             │
│  • ブロックチェーン（タイムスタンプ証明）                   │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: 物理的保護（CHINJU独自）                          │
│  • OTP/eFuse（重要なハッシュを物理的に焼く）                │
│  • データダイオード（変更リクエストを物理的に遮断）         │
│  • 複数拠点分散（単一障害点なし）                           │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 正本の定義（Single Source of Truth）

### 2.1 正本ファイル

| カテゴリ | 正本ファイル | 内容 |
|---------|-------------|------|
| 指標定義 | `chinju/spec/CANONICAL_METRICS.md` | D*, N_eff, SRCI の計算式 |
| 閾値 | `chinju/spec/THRESHOLD_DEFAULTS.md` | デフォルト閾値と変更条件 |
| トレイト | `chinju/traits/hardware_abstraction.md` | 物理層インターフェース |
| エラーコード | `chinju/spec/ERROR_CODES.md` | 標準エラー定義 |

### 2.2 正本の識別子

```rust
/// 正本の識別子
pub struct CanonicalDocument {
    /// 一意識別子（例: "chinju:metrics:v1"）
    pub id: String,
    /// SHA-256 ハッシュ
    pub hash: [u8; 32],
    /// 署名（複数者による閾値署名）
    pub signatures: Vec<ThresholdSignature>,
    /// 有効開始日
    pub effective_from: DateTime<Utc>,
    /// 次バージョンへの移行期限（あれば）
    pub deprecated_by: Option<DateTime<Utc>>,
}
```

---

## 3. 保護メカニズム

### 3.1 署名付きスキーマ

```rust
/// 署名付き定義
pub struct SignedDefinition {
    /// 定義本体（JSON）
    pub content: serde_json::Value,
    /// 定義のハッシュ
    pub content_hash: [u8; 32],
    /// 署名者リスト
    pub signers: Vec<PublicKey>,
    /// 閾値署名（t-of-n）
    pub threshold_signature: ThresholdSignature,
    /// 必要署名数
    pub threshold: usize,
}

impl SignedDefinition {
    /// 署名を検証
    pub fn verify(&self) -> Result<bool, VerifyError> {
        // 1. ハッシュが一致するか
        let computed_hash = sha256(&self.content);
        if computed_hash != self.content_hash {
            return Err(VerifyError::HashMismatch);
        }

        // 2. 署名が有効か
        if !self.threshold_signature.verify(&self.content_hash, &self.signers) {
            return Err(VerifyError::InvalidSignature);
        }

        // 3. 閾値を満たすか
        if self.threshold_signature.signer_count() < self.threshold {
            return Err(VerifyError::InsufficientSigners);
        }

        Ok(true)
    }
}
```

### 3.2 ハッシュ固定（コード埋め込み）

```rust
/// 正本ハッシュ（コンパイル時に固定）
///
/// これを変更するにはコードの変更が必要
/// → PRレビュー → CI/CD → 変更履歴が残る
pub const CANONICAL_HASHES: &[(&str, &str)] = &[
    ("chinju:metrics:v1", "sha256:a1b2c3d4e5f6..."),
    ("chinju:thresholds:v1", "sha256:1a2b3c4d5e6f..."),
    ("chinju:error_codes:v1", "sha256:abcd1234..."),
];

/// 正本の整合性を検証
pub fn verify_canonical_integrity(
    doc_id: &str,
    content: &[u8],
) -> Result<(), IntegrityError> {
    let expected_hash = CANONICAL_HASHES
        .iter()
        .find(|(id, _)| *id == doc_id)
        .map(|(_, hash)| *hash)
        .ok_or(IntegrityError::UnknownDocument)?;

    let actual_hash = format!("sha256:{}", hex::encode(sha256(content)));

    if actual_hash != expected_hash {
        return Err(IntegrityError::HashMismatch {
            expected: expected_hash.to_string(),
            actual: actual_hash,
        });
    }

    Ok(())
}
```

### 3.3 物理層保護（OTP/eFuse）

```rust
/// 重要なハッシュを物理的に焼く
pub trait ImmutableHashStore: Send + Sync {
    /// 正本ハッシュを読み取る
    fn read_canonical_hash(&self, doc_id: &str) -> Result<[u8; 32], Self::Error>;

    /// 正本ハッシュを焼く（一度だけ、不可逆）
    fn burn_canonical_hash(&self, doc_id: &str, hash: [u8; 32]) -> Result<(), Self::Error>;

    /// ハッシュが焼かれているか確認
    fn is_burned(&self, doc_id: &str) -> Result<bool, Self::Error>;
}

/// 物理層での検証フロー
pub fn verify_with_hardware(
    store: &impl ImmutableHashStore,
    doc_id: &str,
    content: &[u8],
) -> Result<(), VerifyError> {
    // eFuseから焼かれたハッシュを読み取り
    let burned_hash = store.read_canonical_hash(doc_id)?;

    // 現在のコンテンツのハッシュを計算
    let current_hash = sha256(content);

    // 一致確認
    if burned_hash != current_hash {
        // ハードウェアレベルで改ざんを検知
        // → 即座に停止
        return Err(VerifyError::HardwareIntegrityFailure);
    }

    Ok(())
}
```

---

## 4. 変更プロセス（RFCベース）

### 4.1 変更が許可されるケース

| ケース | 承認要件 |
|--------|---------|
| タイポ・明確化 | メンテナ1名 |
| 機能追加（後方互換） | メンテナ2名 + RFC |
| 閾値変更 | 閾値署名（3-of-5以上） + RFC |
| 破壊的変更 | 閾値署名（4-of-5以上） + RFC + 移行期間 |

### 4.2 RFCフォーマット

```markdown
# RFC-0001: [Title]

## 概要
[1-2文で変更内容を説明]

## 動機
[なぜこの変更が必要か]

## 影響範囲
[どのコンポーネントが影響を受けるか]

## 互換性
- [ ] 後方互換
- [ ] 前方互換
- [ ] 移行期間: YYYY-MM-DD まで

## セキュリティ考慮
[この変更によるセキュリティへの影響]

## 署名
- [ ] 署名者1: [公開鍵フィンガープリント]
- [ ] 署名者2: [公開鍵フィンガープリント]
- [ ] 署名者3: [公開鍵フィンガープリント]
```

### 4.3 禁止される変更

以下の変更は**いかなる場合も禁止**:

1. **後方非互換な閾値緩和**（セキュリティレベルの低下）
2. **監査ログの削除・省略**（追跡不能化）
3. **署名検証のスキップ**（改ざん検知の無効化）
4. **N_eff ≥ 2 制約の削除**（単一障害点の許容）

---

## 5. 監査証跡（Audit Trail）

### 5.1 変更ログ

```json
{
  "change_id": "CHG-2024-0042",
  "document_id": "chinju:metrics:v1",
  "change_type": "threshold_update",
  "old_value": { "theta_cert": 0.7 },
  "new_value": { "theta_cert": 0.75 },
  "rfc_id": "RFC-0012",
  "signatures": [
    { "signer": "key:abc123", "timestamp": "2024-01-15T10:30:00Z" },
    { "signer": "key:def456", "timestamp": "2024-01-15T11:00:00Z" },
    { "signer": "key:ghi789", "timestamp": "2024-01-15T12:00:00Z" }
  ],
  "effective_from": "2024-02-01T00:00:00Z"
}
```

### 5.2 定期検証

```rust
/// 定期的な整合性検証（推奨: 1時間ごと）
pub async fn periodic_integrity_check(
    canonical_docs: &[CanonicalDocument],
    hardware: &impl ImmutableHashStore,
) -> Vec<IntegrityAlert> {
    let mut alerts = Vec::new();

    for doc in canonical_docs {
        // 1. ファイルシステム上の正本を読み取り
        let content = fs::read(&doc.path).await?;

        // 2. コード埋め込みハッシュと比較
        if let Err(e) = verify_canonical_integrity(&doc.id, &content) {
            alerts.push(IntegrityAlert::CodeEmbeddedMismatch {
                doc_id: doc.id.clone(),
                error: e,
            });
        }

        // 3. 物理層ハッシュと比較
        if let Err(e) = verify_with_hardware(hardware, &doc.id, &content) {
            alerts.push(IntegrityAlert::HardwareMismatch {
                doc_id: doc.id.clone(),
                error: e,
            });
        }

        // 4. 署名検証
        if !doc.verify_signatures() {
            alerts.push(IntegrityAlert::SignatureInvalid {
                doc_id: doc.id.clone(),
            });
        }
    }

    alerts
}
```

---

## 6. 実装優先度

| 優先度 | 保護メカニズム | 実装難易度 | 効果 |
|--------|--------------|-----------|------|
| **P0** | ハッシュ固定（コード埋め込み） | 低 | 変更にPRが必要 |
| **P0** | Git署名付きタグ | 低 | 履歴改ざん防止 |
| **P1** | 署名付きスキーマ | 中 | 複数者合意の証明 |
| **P1** | 定期検証 | 中 | 改ざん検知 |
| **P2** | IPFS永続化 | 中 | 分散保存 |
| **P2** | ブロックチェーン | 高 | タイムスタンプ証明 |
| **P3** | OTP/eFuse書き込み | 高 | 物理的改ざん不可 |

---

## 参照文書

- `spec/00_overview.md` - 全体設計
- `traits/hardware_abstraction.md` - 物理層トレイト
