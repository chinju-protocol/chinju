# Config Schema - 設定スキーマ

## なぜ必要か（背景）

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  設定がバラバラだと...                                       │
│                                                             │
│  • ベンダー間差し替え困難（設定形式が違う）                  │
│  • 検証不可能（何が正しい設定か分からない）                  │
│  • セキュリティホール（設定ミスで脆弱化）                    │
│  • ドキュメント不足（どのパラメータが必須？）                │
│                                                             │
│  ハードウェア差し替え時に設定で詰まる                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 1. 設定ファイル体系

### 1.1 ファイル構成

```
chinju/
├── config/
│   ├── chinju.toml           # メイン設定
│   ├── hardware/
│   │   ├── hsm.toml          # HSM設定
│   │   ├── tee.toml          # TEE設定
│   │   ├── qrng.toml         # QRNG設定
│   │   └── diode.toml        # データダイオード設定
│   ├── thresholds.toml       # 閾値設定
│   └── policy/
│       └── default.toml      # デフォルトポリシー
```

### 1.2 設定の優先順位

```
環境変数 > コマンドライン > ローカル設定 > デフォルト設定
```

---

## 2. メイン設定（chinju.toml）

### 2.1 スキーマ

```toml
# chinju.toml
# CHINJU メイン設定ファイル

# 設定スキーマバージョン（必須）
schema_version = "config_v1"

# ノード識別子（必須）
node_id = "chinju-node-001"

# セキュリティレベル（必須）
# CHJ-0: Development, CHJ-1: Basic, CHJ-2: Enterprise, CHJ-3: Financial, CHJ-4: Sovereign
security_level = "CHJ-2"

# ハードウェア設定（必須）
[hardware]
# HSM設定ファイルパス
hsm_config = "hardware/hsm.toml"
# TEE設定ファイルパス
tee_config = "hardware/tee.toml"
# QRNG設定ファイルパス
qrng_config = "hardware/qrng.toml"
# データダイオード設定ファイルパス（オプション）
diode_config = "hardware/diode.toml"

# 閾値設定（必須）
[thresholds]
# 閾値設定ファイルパス
config_file = "thresholds.toml"
# ハードウェアに焼かれた閾値を優先するか
prefer_hardware = true

# 監査ログ設定
[audit]
# ログ出力先
output_dir = "/var/log/chinju/audit"
# チェーン検証間隔（秒）
chain_verify_interval = 3600
# 署名必須イベント
sign_events = ["tamper_detected", "integrity_violation", "access_revoke"]
# データダイオード経由で送出するか
use_diode = true

# メトリクス設定
[metrics]
# 計測間隔（秒）
measurement_interval = 60
# 定義バージョン
definition_version = "d_star_v1.2"

# トークンバケット設定
[token_bucket]
# 有効化
enabled = true
# 初期残高
initial_balance = 10000
# 最大残高
max_balance = 100000
# 減衰率（秒あたり）
decay_rate = 0.001
```

### 2.2 Rust構造体

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct ChinjuConfig {
    /// スキーマバージョン（必須）
    pub schema_version: String,

    /// ノード識別子（必須）
    pub node_id: String,

    /// セキュリティレベル（必須）
    pub security_level: SecurityLevel,

    /// ハードウェア設定
    pub hardware: HardwareConfig,

    /// 閾値設定
    pub thresholds: ThresholdsConfig,

    /// 監査ログ設定
    #[serde(default)]
    pub audit: AuditConfig,

    /// メトリクス設定
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// トークンバケット設定
    #[serde(default)]
    pub token_bucket: TokenBucketConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityLevel {
    #[serde(rename = "CHJ-0")]
    Development,
    #[serde(rename = "CHJ-1")]
    Basic,
    #[serde(rename = "CHJ-2")]
    Enterprise,
    #[serde(rename = "CHJ-3")]
    Financial,
    #[serde(rename = "CHJ-4")]
    Sovereign,
}
```

---

## 3. HSM設定（hsm.toml）

### 3.1 スキーマ

```toml
# hsm.toml
# HSM設定

schema_version = "hsm_config_v1"

# 実装種別（必須）
# "yubihsm2", "aws_cloudhsm", "azure_keyvault", "gcp_kms", "mock"
implementation = "yubihsm2"

# 有効化（必須）
enabled = true

# YubiHSM2固有設定
[yubihsm2]
# コネクタURL
connector_url = "http://127.0.0.1:12345"
# 認証鍵ID
auth_key_id = 1
# 認証鍵パスワード（環境変数推奨: ${YUBIHSM_PASSWORD}）
password = "${YUBIHSM_PASSWORD}"
# タイムアウト（ミリ秒）
timeout_ms = 5000

# AWS CloudHSM固有設定
[aws_cloudhsm]
cluster_id = "cluster-xxx"
# 認証情報は環境変数またはIAMロールから取得
region = "ap-northeast-1"

# モック設定（開発用）
[mock]
# 決定論的シード（テスト再現用）
seed = 12345
# 遅延シミュレーション（ミリ秒）
delay_ms = 10

# 鍵スロット設定
[[key_slots]]
slot_id = 1
purpose = "root_signing"
algorithm = "ed25519"
exportable = false

[[key_slots]]
slot_id = 2
purpose = "threshold_signing"
algorithm = "ed25519"
exportable = false
```

### 3.2 ベンダー間差し替え

```toml
# 差し替え例: YubiHSM2 → AWS CloudHSM

# Before
implementation = "yubihsm2"
[yubihsm2]
connector_url = "http://127.0.0.1:12345"

# After（他のセクションは触らない）
implementation = "aws_cloudhsm"
[aws_cloudhsm]
cluster_id = "cluster-xxx"
region = "ap-northeast-1"
```

---

## 4. TEE設定（tee.toml）

### 4.1 スキーマ

```toml
# tee.toml
# TEE設定

schema_version = "tee_config_v1"

# 実装種別（必須）
# "intel_sgx", "arm_trustzone", "amd_sev", "aws_nitro", "mock"
implementation = "intel_sgx"

enabled = true

# Intel SGX固有設定
[intel_sgx]
# エンクレーブパス
enclave_path = "/opt/chinju/enclave.signed.so"
# デバッグモード（本番では false）
debug_mode = false
# 最大スレッド数
max_threads = 4
# ヒープサイズ（MB）
heap_size_mb = 256
# スタックサイズ（KB）
stack_size_kb = 256

# AWS Nitro固有設定
[aws_nitro]
# エンクレーブイメージ
enclave_cid = 16
# メモリ（MB）
memory_mb = 512
# vCPU数
vcpu_count = 2

# アテステーション設定
[attestation]
# 検証プロバイダ
provider = "intel_ias"  # or "dcap", "aws_nitro_attestation"
# SPID（IASの場合）
spid = "${SGX_SPID}"
# API鍵
api_key = "${SGX_API_KEY}"
# キャッシュ有効期間（秒）
cache_duration = 3600
```

---

## 5. QRNG設定（qrng.toml）

### 5.1 スキーマ

```toml
# qrng.toml
# QRNG設定

schema_version = "qrng_config_v1"

# 実装種別（必須）
# "id_quantique", "quside", "intel_rdrand", "mock"
implementation = "id_quantique"

enabled = true

# ID Quantique固有設定
[id_quantique]
# デバイスパス
device_path = "/dev/quantis0"
# 最小エントロピー（ビット/バイト）
min_entropy = 7.9
# ヘルスチェック間隔（秒）
health_check_interval = 60

# Intel RDRAND固有設定（フォールバック）
[intel_rdrand]
# RDSEEDも使用するか
use_rdseed = true
# 最小エントロピー推定
min_entropy = 7.5

# エントロピープール設定
[entropy_pool]
# プールサイズ（バイト）
size = 4096
# 再シード閾値（バイト）
reseed_threshold = 1024
# 混合アルゴリズム
mixing_algorithm = "sha512"
```

---

## 6. 閾値設定（thresholds.toml）

### 6.1 スキーマ

```toml
# thresholds.toml
# 閾値設定

schema_version = "thresholds_v1"

# D* 閾値
[d_star]
# 警告閾値
warn = 0.6
# 制限閾値
limit = 0.7
# 停止閾値
halt = 0.8

# N_eff 閾値
[n_eff]
# 最小実効数
min = 2.0
# 警告閾値
warn = 2.5

# SRCI 閾値
[srci]
# 異常下限
anomaly_low = -0.5
# 異常上限
anomaly_high = 0.8

# トークン閾値
[tokens]
# 警告残高
warn_balance = 1000
# 最小残高（これを下回ると停止）
min_balance = 0

# 人間能力閾値（HCAL）
[hcal]
# 認定閾値
certification = 0.7
# 失効閾値
revocation = 0.5
```

### 6.2 ハードウェア焼き込み閾値との関係

```rust
/// 閾値の解決
pub fn resolve_threshold(
    config_threshold: f64,
    hardware_threshold: Option<f64>,
    prefer_hardware: bool,
) -> f64 {
    match (hardware_threshold, prefer_hardware) {
        // ハードウェア閾値があり、優先する場合
        (Some(hw), true) => hw,
        // ハードウェア閾値がない、または優先しない場合
        _ => config_threshold,
    }
}
```

---

## 7. 検証

### 7.1 設定バリデーション

```rust
impl ChinjuConfig {
    pub fn validate(&self) -> Result<(), Vec<ConfigError>> {
        let mut errors = Vec::new();

        // スキーマバージョン確認
        if !self.schema_version.starts_with("config_v") {
            errors.push(ConfigError::InvalidSchemaVersion(self.schema_version.clone()));
        }

        // セキュリティレベルに応じたハードウェア要件
        match self.security_level {
            SecurityLevel::Financial | SecurityLevel::Sovereign => {
                // CHJ-3以上はハードウェア必須
                if !self.hardware.hsm_config.is_some() {
                    errors.push(ConfigError::HsmRequired);
                }
                if !self.hardware.tee_config.is_some() {
                    errors.push(ConfigError::TeeRequired);
                }
            }
            SecurityLevel::Enterprise => {
                // CHJ-2はHSM必須
                if !self.hardware.hsm_config.is_some() {
                    errors.push(ConfigError::HsmRequired);
                }
            }
            _ => {}
        }

        // 閾値の整合性
        if self.thresholds.d_star.warn >= self.thresholds.d_star.halt {
            errors.push(ConfigError::InvalidThreshold("d_star.warn >= d_star.halt".into()));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

### 7.2 JSON Schemaでの検証

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://chinju.org/schemas/config_v1.json",
  "title": "CHINJU Configuration",
  "type": "object",
  "required": ["schema_version", "node_id", "security_level", "hardware", "thresholds"],
  "properties": {
    "schema_version": {
      "type": "string",
      "pattern": "^config_v[0-9]+$"
    },
    "node_id": {
      "type": "string",
      "minLength": 1
    },
    "security_level": {
      "type": "string",
      "enum": ["CHJ-0", "CHJ-1", "CHJ-2", "CHJ-3", "CHJ-4"]
    }
  }
}
```

---

## 8. 環境変数展開

### 8.1 構文

```toml
# ${VAR_NAME} で環境変数を参照
password = "${YUBIHSM_PASSWORD}"

# デフォルト値付き
region = "${AWS_REGION:-ap-northeast-1}"
```

### 8.2 実装

```rust
use regex::Regex;
use std::env;

pub fn expand_env_vars(value: &str) -> String {
    let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)(?::-([^}]*))?\}").unwrap();

    re.replace_all(value, |caps: &regex::Captures| {
        let var_name = &caps[1];
        let default = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        env::var(var_name).unwrap_or_else(|_| default.to_string())
    }).to_string()
}
```

---

## 9. 設定のマイグレーション

### 9.1 バージョン間のマイグレーション

```rust
/// 設定のマイグレーション
pub fn migrate_config(old_config: &str, from_version: &str, to_version: &str) -> Result<String, MigrateError> {
    match (from_version, to_version) {
        ("config_v1", "config_v2") => migrate_v1_to_v2(old_config),
        _ => Err(MigrateError::UnsupportedMigration),
    }
}

fn migrate_v1_to_v2(old: &str) -> Result<String, MigrateError> {
    let mut config: toml::Value = toml::from_str(old)?;

    // バージョン更新
    config["schema_version"] = toml::Value::String("config_v2".to_string());

    // 新規必須フィールドにデフォルト値
    if !config.get("observability").is_some() {
        config["observability"] = toml::toml! {
            enabled = true
            otlp_endpoint = "http://localhost:4317"
        };
    }

    Ok(toml::to_string_pretty(&config)?)
}
```

---

## 参照文書

- `chinju/spec/VERSIONING.md` - バージョン管理
- `chinju/spec/ERROR_CODES.md` - エラーコード
- `chinju/hardware/SECURITY_LEVELS.md` - セキュリティレベル定義
- `chinju/hardware/INTEGRATION_GUIDE.md` - パートナー統合ガイド
