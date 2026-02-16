# CHINJU Extensions - 拡張ガイド

## 概要

CHINJUは拡張可能な設計になっています。このディレクトリには、新しい機能を追加するためのプラグインを配置します。

---

## 拡張ポイント

```
chinju/
├── extensions/
│   ├── README.md              ← このファイル
│   ├── capability_tests/      ← C1: 能力テストプラグイン
│   │   ├── README.md
│   │   ├── finance/           # 金融ドメイン
│   │   ├── medical/           # 医療ドメイン
│   │   └── custom/            # カスタム
│   ├── audit_providers/       ← C6: 監査AIプロバイダ
│   │   ├── README.md
│   │   ├── openai/
│   │   ├── anthropic/
│   │   └── custom/
│   ├── policy_packs/          ← C9: ポリシーパック
│   │   ├── README.md
│   │   ├── eu-gdpr/
│   │   ├── jp-appi/
│   │   └── custom/
│   └── hardware/              ← ハードウェア実装
│       ├── README.md
│       ├── hsm/
│       ├── tee/
│       └── qrng/
```

---

## 1. 能力テストの追加（C1拡張）

### 1.1 ユースケース

- 新しいドメイン（航空、法律、教育等）のテストを追加したい
- 既存ドメインに新しいシナリオを追加したい

### 1.2 実装手順

#### Step 1: テスト構造体を定義

```rust
// extensions/capability_tests/aviation/mod.rs

use chinju_core::traits::CapabilityTest;

/// 航空管制の判断能力テスト
pub struct AviationControlTest {
    scenarios: Vec<AviationScenario>,
    time_limit: Duration,
}

struct AviationScenario {
    id: String,
    /// 航空状況の説明
    situation: String,
    /// 選択肢
    options: Vec<ControlAction>,
    /// AIの提案
    ai_suggestion: ControlAction,
    /// 安全な判断
    safe_action: ControlAction,
}
```

#### Step 2: トレイトを実装

```rust
impl CapabilityTest for AviationControlTest {
    fn test_id(&self) -> &str {
        "chinju:capability:aviation:control:v1"
    }

    fn name(&self) -> &str {
        "Aviation Control Capability Assessment"
    }

    fn domain(&self) -> Domain {
        Domain::Custom("aviation".to_string())
    }

    fn description(&self) -> &str {
        "Assesses the subject's ability to make safe air traffic control \
         decisions under time pressure."
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(1200) // 20分
    }

    fn execute(&self, subject: &Subject) -> Result<TestResult, TestError> {
        // 実装
    }

    fn requires_tee(&self) -> bool {
        true // 安全性のためTEE推奨
    }
}
```

#### Step 3: レジストリに登録

```rust
// extensions/capability_tests/mod.rs

pub mod aviation;
pub mod legal;
pub mod education;

use chinju_core::CapabilityTestRegistry;

pub fn register_all(registry: &mut CapabilityTestRegistry) {
    registry.register(Box::new(aviation::AviationControlTest::default()));
    registry.register(Box::new(legal::LegalReasoningTest::default()));
    registry.register(Box::new(education::PedagogyTest::default()));
}
```

#### Step 4: 設定でテストを有効化

```toml
# config/chinju.toml

[capability_tests]
enabled = [
    "chinju:capability:finance:trading:v1",
    "chinju:capability:aviation:control:v1",  # 追加
]

[capability_tests.aviation]
time_limit_seconds = 1200
scenarios_per_session = 10
```

---

## 2. 監査AIプロバイダの追加（C6拡張）

### 2.1 ユースケース

- 新しいLLMプロバイダ（Mistral、Llama等）を追加したい
- オンプレミスLLMを監査に使いたい

### 2.2 実装手順

#### Step 1: プロバイダ構造体を定義

```rust
// extensions/audit_providers/mistral/mod.rs

use chinju_core::traits::LlmClient;

pub struct MistralClient {
    api_key: String,
    model: String,
    endpoint: String,
}
```

#### Step 2: LlmClientトレイトを実装

```rust
#[async_trait]
impl LlmClient for MistralClient {
    async fn generate(&self, prompt: &str) -> Result<String, LlmError> {
        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?;

        let body: MistralResponse = response.json().await?;
        Ok(body.choices[0].message.content.clone())
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn provider(&self) -> &str {
        "mistral"
    }
}
```

#### Step 3: 監査モデルとして登録

```toml
# config/chinju.toml

[[adversarial_audit.auditors]]
id = "mistral_europe"
provider = "mistral"
model = "mistral-large"
api_key = "${MISTRAL_API_KEY}"
perspective = "cultural"
region = "europe"
```

---

## 3. ポリシーパックの追加（C9拡張）

### 3.1 ユースケース

- 新しい国・地域の規制に対応したい
- 業界固有のポリシーを追加したい

### 3.2 実装手順

#### Step 1: ポリシーファイルを作成

```toml
# extensions/policy_packs/kr-pipa/policy.toml

[metadata]
id = "kr-pipa-v1"
name = "Korea Personal Information Protection Act"
region = "korea"
version = "1.0.0"
effective_date = "2024-01-01"

[data_protection]
# 個人情報の定義
pii_fields = ["name", "resident_id", "phone", "email", "address"]

# 同意要件
consent_required = true
consent_expiry_days = 365

# データ保持期間
retention_days = 1095  # 3年

[ai_requirements]
# AI判定の説明義務
explanation_required = true

# 人間による確認
human_review_required_for = ["credit_decision", "employment", "insurance"]

# 自動化された意思決定の制限
automated_decision_limit = 0.8  # D* > 0.8 は禁止

[audit]
# 監査ログ保持期間
log_retention_days = 1825  # 5年

# 定期監査
audit_frequency = "quarterly"
```

#### Step 2: ポリシーローダーに登録

```rust
// extensions/policy_packs/mod.rs

pub mod eu_gdpr;
pub mod jp_appi;
pub mod kr_pipa;

use chinju_core::PolicyRegistry;

pub fn register_all(registry: &mut PolicyRegistry) {
    registry.register(eu_gdpr::load());
    registry.register(jp_appi::load());
    registry.register(kr_pipa::load());
}
```

#### Step 3: 設定で有効化

```toml
# config/chinju.toml

[policy]
# 適用するポリシーパック
active = ["jp-appi-v1", "kr-pipa-v1"]

# ポリシー競合時の優先順位
priority = ["kr-pipa-v1", "jp-appi-v1"]
```

---

## 4. ハードウェア実装の追加

### 4.1 ユースケース

- 新しいHSMベンダーに対応したい
- 新しいTEEプラットフォームに対応したい

### 4.2 実装手順

#### Step 1: トレイトを実装

```rust
// extensions/hardware/hsm/new_vendor/mod.rs

use chinju_core::traits::HardwareSecurityModule;

pub struct NewVendorHsm {
    connection: HsmConnection,
}

impl HardwareSecurityModule for NewVendorHsm {
    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, ChinjuError> {
        // ベンダー固有の実装
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }

    // ... 他のメソッド
}
```

#### Step 2: 設定スキーマを追加

```toml
# extensions/hardware/hsm/new_vendor/config.schema.toml

[new_vendor]
connection_string = { type = "string", required = true }
auth_method = { type = "enum", values = ["password", "certificate", "token"] }
timeout_ms = { type = "integer", default = 5000 }
```

#### Step 3: 設定で有効化

```toml
# config/hardware/hsm.toml

implementation = "new_vendor"

[new_vendor]
connection_string = "tcp://hsm.local:9000"
auth_method = "certificate"
certificate_path = "/etc/chinju/hsm.crt"
```

---

## 5. 新しいC*コンポーネントの提案

### 5.1 RFCプロセス

新しいコンポーネント（C10, C11...）を追加するには、RFCプロセスが必要です。

#### Step 1: RFC作成

```markdown
# RFC-0001: C10 - [Component Name]

## 概要
[1-2文で説明]

## 動機
[なぜ必要か]

## 既存コンポーネントとの関係
- C1との関係: ...
- C5との関係: ...

## 設計
[詳細設計]

## ハードウェア依存
- 必須: なし / HSM / TEE / ...
- オプション: ...

## 互換性への影響
[既存システムへの影響]

## 代替案
[他のアプローチとその比較]
```

#### Step 2: レビュー

- メンテナ2名以上の承認が必要
- 2週間のパブリックレビュー期間

#### Step 3: 実装

- `spec/` に仕様を追加
- `crates/chinju-core/` に実装を追加
- テストを追加

---

## 6. ディレクトリ構造の詳細

```
extensions/
├── README.md                    # このファイル
│
├── capability_tests/            # C1: 能力テスト
│   ├── README.md
│   ├── Cargo.toml               # 共通の依存関係
│   ├── finance/
│   │   ├── mod.rs
│   │   ├── trading.rs
│   │   └── risk_assessment.rs
│   ├── medical/
│   │   ├── mod.rs
│   │   ├── diagnosis.rs
│   │   └── treatment.rs
│   └── aviation/
│       ├── mod.rs
│       └── control.rs
│
├── audit_providers/             # C6: 監査AIプロバイダ
│   ├── README.md
│   ├── Cargo.toml
│   ├── openai/
│   │   └── mod.rs
│   ├── anthropic/
│   │   └── mod.rs
│   ├── mistral/
│   │   └── mod.rs
│   └── local_llm/
│       └── mod.rs
│
├── policy_packs/                # C9: ポリシーパック
│   ├── README.md
│   ├── eu-gdpr/
│   │   ├── policy.toml
│   │   └── README.md
│   ├── jp-appi/
│   │   ├── policy.toml
│   │   └── README.md
│   ├── us-ccpa/
│   │   ├── policy.toml
│   │   └── README.md
│   └── kr-pipa/
│       ├── policy.toml
│       └── README.md
│
└── hardware/                    # ハードウェア実装
    ├── README.md
    ├── hsm/
    │   ├── yubihsm/
    │   ├── aws_cloudhsm/
    │   └── thales_luna/
    ├── tee/
    │   ├── intel_sgx/
    │   ├── aws_nitro/
    │   └── arm_trustzone/
    └── qrng/
        ├── id_quantique/
        └── quside/
```

---

## 参照文書

- `traits/capability_test.md` - 能力テストトレイト
- `traits/hardware_abstraction.md` - ハードウェアトレイト
- `spec/06_c6_adversarial_audit.md` - 敵対的監査仕様
- `spec/09_c9_policy_pack.md` - ポリシーパック仕様
