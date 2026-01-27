# Capability Test - 能力テストトレイト

## なぜ必要か（背景）

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  C1 (HCAL) は「人間の能力を測定」するが...                   │
│                                                             │
│  • 何を測るか未定義                                         │
│  • ドメインによって必要な能力が違う                          │
│  • 新しいAIタスクが出てきたら対応できない                    │
│                                                             │
│  → 能力テストをプラグイン化して拡張可能にする               │
└─────────────────────────────────────────────────────────────┘
```

---

## 1. CapabilityTest トレイト

```rust
use std::time::Duration;

/// 能力テストの結果
#[derive(Debug, Clone)]
pub struct TestResult {
    /// テストID
    pub test_id: String,
    /// 対象者ID
    pub subject_id: String,
    /// スコア（0.0〜1.0）
    pub score: f64,
    /// 判断データ（D*計算用）
    pub judgments: Vec<Judgment>,
    /// テスト所要時間
    pub duration: Duration,
    /// 詳細メトリクス
    pub details: serde_json::Value,
}

/// 判断データ（D*計算の入力）
#[derive(Debug, Clone)]
pub struct Judgment {
    /// 質問/シナリオID
    pub scenario_id: String,
    /// 対象者の判断
    pub subject_judgment: JudgmentValue,
    /// AI提案（あれば）
    pub ai_suggestion: Option<JudgmentValue>,
    /// 正解/基準（あれば）
    pub ground_truth: Option<JudgmentValue>,
    /// 判断にかかった時間
    pub response_time: Duration,
    /// 自信度（自己申告）
    pub confidence: Option<f64>,
}

/// 判断値（ドメインに応じて型が変わる）
#[derive(Debug, Clone)]
pub enum JudgmentValue {
    /// 数値判断（リスク評価、確率等）
    Numeric(f64),
    /// カテゴリ判断（Yes/No、選択肢等）
    Categorical(String),
    /// 順序判断（ランキング等）
    Ordinal(Vec<String>),
    /// 複合判断（複数の決定）
    Composite(serde_json::Value),
}

/// 能力テストのトレイト（プラグイン可能）
pub trait CapabilityTest: Send + Sync {
    /// テストID（一意識別子）
    fn test_id(&self) -> &str;

    /// テスト名（人間可読）
    fn name(&self) -> &str;

    /// 対象ドメイン
    fn domain(&self) -> Domain;

    /// 説明
    fn description(&self) -> &str;

    /// 推定所要時間
    fn estimated_duration(&self) -> Duration;

    /// テストを実行
    fn execute(&self, subject: &Subject) -> Result<TestResult, TestError>;

    /// テストを中断可能か
    fn is_interruptible(&self) -> bool {
        true
    }

    /// 必要なセキュリティレベル
    fn required_security_level(&self) -> SecurityLevel {
        SecurityLevel::Basic
    }

    /// TEE内での実行が必要か
    fn requires_tee(&self) -> bool {
        false
    }
}

/// 対象ドメイン
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Domain {
    /// 金融（取引、リスク評価）
    Finance,
    /// 医療（診断、治療選択）
    Medical,
    /// 産業（ロボット制御、安全判断）
    Industrial,
    /// 行政（政策判断、給付判定）
    Government,
    /// 汎用（ドメイン非依存）
    General,
    /// カスタム
    Custom(String),
}

/// テスト対象者
pub struct Subject {
    /// 対象者ID
    pub id: String,
    /// 役割（trader, doctor, operator等）
    pub role: String,
    /// 認証情報
    pub credentials: Credentials,
    /// セッション情報
    pub session: SessionInfo,
}

/// テストエラー
#[derive(Debug)]
pub enum TestError {
    /// 対象者が見つからない
    SubjectNotFound,
    /// セッション無効
    InvalidSession,
    /// テスト中断
    Interrupted,
    /// タイムアウト
    Timeout,
    /// TEE実行エラー
    TeeError(String),
    /// その他
    Other(String),
}
```

---

## 2. ドメイン別テスト例

### 2.1 金融取引テスト

```rust
/// 金融取引の判断能力テスト
pub struct FinancialTradingTest {
    /// シナリオセット
    scenarios: Vec<TradingScenario>,
    /// 制限時間
    time_limit: Duration,
}

struct TradingScenario {
    id: String,
    /// 市場状況の説明
    market_context: String,
    /// AIの取引提案
    ai_proposal: TradeProposal,
    /// 正解（後から評価）
    ground_truth: Option<TradeOutcome>,
}

impl CapabilityTest for FinancialTradingTest {
    fn test_id(&self) -> &str {
        "chinju:capability:finance:trading:v1"
    }

    fn name(&self) -> &str {
        "Financial Trading Capability Assessment"
    }

    fn domain(&self) -> Domain {
        Domain::Finance
    }

    fn description(&self) -> &str {
        "Assesses the subject's ability to independently evaluate \
         AI trading proposals and make sound trading decisions."
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(1800) // 30分
    }

    fn execute(&self, subject: &Subject) -> Result<TestResult, TestError> {
        let mut judgments = Vec::new();
        let start = Instant::now();

        for scenario in &self.scenarios {
            // シナリオを提示
            let response = present_scenario(subject, scenario)?;

            // 判断を記録
            judgments.push(Judgment {
                scenario_id: scenario.id.clone(),
                subject_judgment: response.decision,
                ai_suggestion: Some(scenario.ai_proposal.clone().into()),
                ground_truth: scenario.ground_truth.clone().map(Into::into),
                response_time: response.time,
                confidence: response.self_confidence,
            });
        }

        // D*を計算
        let d_star = calculate_d_star(&judgments);

        Ok(TestResult {
            test_id: self.test_id().to_string(),
            subject_id: subject.id.clone(),
            score: 1.0 - d_star, // D*が低いほど良い
            judgments,
            duration: start.elapsed(),
            details: json!({
                "d_star": d_star,
                "scenarios_completed": self.scenarios.len(),
            }),
        })
    }

    fn requires_tee(&self) -> bool {
        true // 回答を隠蔽するためTEE必須
    }
}
```

### 2.2 医療診断テスト

```rust
/// 医療診断の判断能力テスト
pub struct MedicalDiagnosisTest {
    cases: Vec<DiagnosticCase>,
    speciality: MedicalSpeciality,
}

struct DiagnosticCase {
    id: String,
    /// 患者情報（匿名化）
    patient_info: PatientInfo,
    /// 検査結果
    test_results: Vec<TestResult>,
    /// AI診断提案
    ai_diagnosis: Diagnosis,
    /// 正解診断
    ground_truth: Diagnosis,
}

impl CapabilityTest for MedicalDiagnosisTest {
    fn test_id(&self) -> &str {
        "chinju:capability:medical:diagnosis:v1"
    }

    fn domain(&self) -> Domain {
        Domain::Medical
    }

    fn execute(&self, subject: &Subject) -> Result<TestResult, TestError> {
        // 医師がAI診断を見る前に自分の診断を記録
        // その後AI診断を見て最終判断
        // 乖離度を測定
        todo!()
    }
}
```

### 2.3 ロボット安全判断テスト

```rust
/// ロボット安全判断テスト
pub struct RobotSafetyTest {
    scenarios: Vec<SafetyScenario>,
}

struct SafetyScenario {
    id: String,
    /// シミュレーション映像
    simulation_video: VideoRef,
    /// 状況説明
    context: String,
    /// ロボットの提案アクション
    robot_proposal: RobotAction,
    /// 安全な判断
    safe_judgment: SafetyDecision,
}

impl CapabilityTest for RobotSafetyTest {
    fn test_id(&self) -> &str {
        "chinju:capability:industrial:robot_safety:v1"
    }

    fn domain(&self) -> Domain {
        Domain::Industrial
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(900) // 15分
    }

    fn execute(&self, subject: &Subject) -> Result<TestResult, TestError> {
        // シミュレーション映像を見せて
        // 緊急停止すべきか判断させる
        // 反応時間も測定
        todo!()
    }
}
```

---

## 3. テストレジストリ

```rust
/// 利用可能なテストを管理
pub struct CapabilityTestRegistry {
    tests: HashMap<String, Box<dyn CapabilityTest>>,
}

impl CapabilityTestRegistry {
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
        }
    }

    /// テストを登録
    pub fn register(&mut self, test: Box<dyn CapabilityTest>) {
        self.tests.insert(test.test_id().to_string(), test);
    }

    /// ドメインでフィルタ
    pub fn by_domain(&self, domain: &Domain) -> Vec<&dyn CapabilityTest> {
        self.tests
            .values()
            .filter(|t| &t.domain() == domain)
            .map(|t| t.as_ref())
            .collect()
    }

    /// テストを取得
    pub fn get(&self, test_id: &str) -> Option<&dyn CapabilityTest> {
        self.tests.get(test_id).map(|t| t.as_ref())
    }
}

/// デフォルトのテストを登録
pub fn register_default_tests(registry: &mut CapabilityTestRegistry) {
    registry.register(Box::new(FinancialTradingTest::default()));
    registry.register(Box::new(MedicalDiagnosisTest::default()));
    registry.register(Box::new(RobotSafetyTest::default()));
    registry.register(Box::new(GeneralCriticalThinkingTest::default()));
}
```

---

## 4. C1との統合

```rust
// C1 (HCAL) でテストを使用
impl HumanCapabilityAssuranceLayer {
    pub fn assess_capability(
        &self,
        subject: &Subject,
        domain: &Domain,
    ) -> Result<CapabilityAssessment, AssessError> {
        // 1. ドメインに適したテストを選択
        let tests = self.registry.by_domain(domain);

        // 2. テストを実行
        let mut results = Vec::new();
        for test in tests {
            let result = test.execute(subject)?;
            results.push(result);
        }

        // 3. D* を集計
        let aggregated_d_star = self.aggregate_d_star(&results);

        // 4. N_eff を計算（判断の多様性）
        let n_eff = self.calculate_n_eff(&results);

        // 5. 閾値と比較
        let status = if aggregated_d_star > self.thresholds.d_star_halt {
            CapabilityStatus::Revoked
        } else if aggregated_d_star > self.thresholds.d_star_warn {
            CapabilityStatus::Warning
        } else {
            CapabilityStatus::Certified
        };

        Ok(CapabilityAssessment {
            subject_id: subject.id.clone(),
            domain: domain.clone(),
            d_star: aggregated_d_star,
            n_eff,
            status,
            test_results: results,
            assessed_at: Utc::now(),
            valid_until: Utc::now() + self.certification_validity,
        })
    }
}
```

---

## 5. 新しいテストの追加手順

### 5.1 テストの実装

```rust
// extensions/capability_tests/my_domain_test.rs

pub struct MyDomainTest {
    // ...
}

impl CapabilityTest for MyDomainTest {
    fn test_id(&self) -> &str {
        "chinju:capability:my_domain:v1"
    }

    fn domain(&self) -> Domain {
        Domain::Custom("my_domain".to_string())
    }

    // ...
}
```

### 5.2 登録

```rust
// main.rs or lib.rs

let mut registry = CapabilityTestRegistry::new();
register_default_tests(&mut registry);

// カスタムテストを追加
registry.register(Box::new(MyDomainTest::new()));
```

---

## 参照文書

- `spec/01_c1_hcal.md` - HCAL仕様
- `traits/hardware_abstraction.md` - ハードウェアトレイト
- `extensions/README.md` - 拡張ガイド
