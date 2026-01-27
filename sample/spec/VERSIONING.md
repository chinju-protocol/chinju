# Versioning - バージョン管理仕様

## なぜ必要か（背景）

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  バージョン情報がないと...                                   │
│                                                             │
│  • 互換性が分からない（古いクライアントが動くか？）          │
│  • デバッグが困難（どのバージョンでバグが入った？）          │
│  • ロールバック不可（どこに戻せばいい？）                    │
│  • 監査不可能（いつ変更された？）                            │
│                                                             │
│  外部システム接続時に致命的                                  │
│  → バージョンが合わないとデータ交換できない                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 1. バージョン体系

### 1.1 バージョン識別子

```
chinju:<component>:<version>

例:
  chinju:core:v1.2.3
  chinju:metrics:v1
  chinju:hardware:tee:v2
  chinju:audit_log:v1
```

### 1.2 セマンティックバージョニング

```
MAJOR.MINOR.PATCH

MAJOR: 破壊的変更（後方互換なし）
MINOR: 機能追加（後方互換あり）
PATCH: バグ修正・セキュリティパッチ
```

### 1.3 スキーマバージョン（簡易形式）

スキーマ・プロトコル定義には簡易形式を使用：

```
<name>_v<major>

例:
  metrics_v1
  audit_log_v2
  hardware_config_v1
```

---

## 2. ペイロードの必須フィールド

### 2.1 すべてのペイロードに必須

```rust
/// 必須メタデータ
pub struct RequiredMetadata {
    /// スキーマバージョン（例: "metrics_v1"）
    pub schema_version: String,

    /// ペイロード種別（複数種別がある場合）
    pub payload_type: Option<String>,

    /// 定義バージョン（例: "d_star_v1.2"）
    pub definition_version: String,

    /// 生成タイムスタンプ
    pub timestamp: DateTime<Utc>,

    /// 生成元識別子
    pub source_id: String,
}
```

### 2.2 JSON例

```json
{
  "schema_version": "metrics_v1",
  "payload_type": "d_star_measurement",
  "definition_version": "d_star_v1.2",
  "timestamp": "2024-01-15T10:30:00Z",
  "source_id": "chinju-core-node-001",
  "data": {
    "d_star": 0.73,
    "n_eff": 2.1,
    "srci": 0.45
  }
}
```

---

## 3. 互換性ポリシー

### 3.1 互換性マトリクス

```json
// chinju/compatibility.json
{
  "metrics_v1": {
    "accepts": ["metrics_v1"],
    "maps_to": "metrics_v1",
    "deprecated_since": null
  },
  "metrics_v2": {
    "accepts": ["metrics_v1", "metrics_v2"],
    "maps_to": "metrics_v2",
    "deprecated_since": null
  },
  "audit_log_v1": {
    "accepts": ["audit_log_v1"],
    "maps_to": "audit_log_v1",
    "deprecated_since": "2024-06-01"
  }
}
```

### 3.2 バージョンネゴシエーション

```rust
/// バージョンネゴシエーション
pub struct VersionNegotiator {
    compatibility: HashMap<String, CompatibilityEntry>,
}

impl VersionNegotiator {
    /// クライアントのバージョンを受け入れるか判定
    pub fn accepts(&self, client_version: &str) -> Result<String, NegotiationError> {
        for (current, entry) in &self.compatibility {
            if entry.accepts.contains(&client_version.to_string()) {
                // 内部バージョンにマッピング
                return Ok(entry.maps_to.clone());
            }
        }
        Err(NegotiationError::UnsupportedVersion(client_version.to_string()))
    }

    /// 非推奨警告
    pub fn deprecation_warning(&self, version: &str) -> Option<String> {
        self.compatibility.get(version).and_then(|entry| {
            entry.deprecated_since.map(|date| {
                format!(
                    "Version {} is deprecated since {}. Please migrate.",
                    version, date
                )
            })
        })
    }
}
```

### 3.3 破壊的変更のルール

| 変更種別 | MAJOR増加 | 移行期間 |
|---------|----------|---------|
| フィールド削除 | 必須 | 6ヶ月 |
| フィールド型変更 | 必須 | 6ヶ月 |
| 計算式変更 | 必須 | 3ヶ月 |
| フィールド追加 | 不要 | - |
| オプション化 | 不要 | - |

---

## 4. 外部システムとの互換性

### 4.1 指標定義

CHINJUは以下の指標を提供：

| 指標 | バージョン | 説明 |
|-----|---------|-------|
| D* | `chinju:metrics:d_star_v1` | 乖離度 |
| N_eff | `chinju:metrics:n_eff_v1` | 実効選択肢数 |
| SRCI | `chinju:metrics:srci_v1` | 修復サイクル指数 |

### 4.2 相互運用フロー

```
┌─────────────────────────────────────────────────────────────┐
│  外部システム接続時                                          │
│                                                             │
│  1. バージョン交換                                           │
│     CHINJU:  "I speak metrics_v1, audit_log_v2"             │
│     External: "I accept metrics_v1"                         │
│                                                             │
│  2. 共通バージョン決定                                       │
│     → metrics_v1 で通信                                     │
│                                                             │
│  3. データ交換                                               │
│     CHINJU → { schema_version: "metrics_v1", d_star: 0.73 } │
│     External ← パース成功、処理                             │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 定義バージョンの追跡

```rust
/// 定義バージョンをログに必ず記録
pub struct MetricsMeasurement {
    /// 使用した定義のバージョン
    pub definition_version: String,
    /// 計算結果
    pub d_star: f64,
    pub n_eff: f64,
    pub srci: f64,
}

impl MetricsMeasurement {
    pub fn to_log_entry(&self) -> serde_json::Value {
        json!({
            "definition_version": self.definition_version,  // 必須
            "d_star": self.d_star,
            "n_eff": self.n_eff,
            "srci": self.srci,
            "timestamp": Utc::now().to_rfc3339()
        })
    }
}
```

---

## 5. マイグレーション

### 5.1 マイグレーションスクリプト

```rust
/// v1 → v2 のマイグレーション
pub fn migrate_metrics_v1_to_v2(v1: MetricsV1) -> MetricsV2 {
    MetricsV2 {
        // 既存フィールドをそのまま
        d_star: v1.d_star,
        n_eff: v1.n_eff,
        srci: v1.srci,
        // 新規フィールドはデフォルト値
        confidence: None,
        source_quality: SourceQuality::Unknown,
        // バージョン情報を更新
        schema_version: "metrics_v2".to_string(),
    }
}
```

### 5.2 互換性レイヤー

```rust
/// 複数バージョンを受け付けるデシリアライザ
pub fn deserialize_metrics(raw: &[u8]) -> Result<MetricsCurrent, DeserializeError> {
    // まずバージョンだけ読む
    let version_probe: VersionProbe = serde_json::from_slice(raw)?;

    match version_probe.schema_version.as_str() {
        "metrics_v1" => {
            let v1: MetricsV1 = serde_json::from_slice(raw)?;
            Ok(migrate_metrics_v1_to_v2(v1).into())
        }
        "metrics_v2" => {
            let v2: MetricsV2 = serde_json::from_slice(raw)?;
            Ok(v2.into())
        }
        unknown => Err(DeserializeError::UnsupportedVersion(unknown.to_string())),
    }
}
```

---

## 6. 検証ルール

### 6.1 CIでの検証

```yaml
# .github/workflows/version-check.yml
- name: Check version consistency
  run: |
    # すべてのスキーマにバージョンがあるか
    chinju-cli schema validate --require-version

    # 互換性マトリクスが最新か
    chinju-cli compat check

    # 非推奨バージョンの使用警告
    chinju-cli compat deprecation-report
```

### 6.2 ランタイム検証

```rust
/// ペイロードの必須フィールド検証
pub fn validate_payload(payload: &serde_json::Value) -> Result<(), ValidationError> {
    // schema_version 必須
    if !payload.get("schema_version").is_some() {
        return Err(ValidationError::MissingSchemaVersion);
    }

    // definition_version 必須（計測データの場合）
    if payload.get("d_star").is_some() || payload.get("n_eff").is_some() {
        if !payload.get("definition_version").is_some() {
            return Err(ValidationError::MissingDefinitionVersion);
        }
    }

    Ok(())
}
```

---

## 参照文書

- `spec/PROTOCOL_PROTECTION.md` - プロトコル保護
