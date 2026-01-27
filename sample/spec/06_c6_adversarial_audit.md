# C6: 敵対的監査による出力均衡化システム - 詳細設計

## 1. 目的

主モデルの出力に対して、異なるバイアスや価値観を持つ監査モデルによる検証・修正を義務付け、出力の偏りや危険性を構造的に中和する。

## 2. 核心アイデア

- 単一モデルによる独占的な情報出力を防ぐ
- 異なる文化圏・専門領域の監査モデルが相互チェック
- 意味的乖離が大きい場合は両論併記
- 来歴情報（Provenance）で透明性を確保

## 3. ソフトウェア構成

```
c6_adversarial_audit/
├── primary/
│   ├── mod.rs
│   └── generator.rs       # 主モデル連携
├── auditor/
│   ├── mod.rs
│   ├── registry.rs        # 監査モデル登録
│   ├── selector.rs        # 監査モデル選択
│   └── executor.rs        # 監査実行
├── critique/
│   ├── mod.rs
│   ├── bias.rs            # バイアス検出
│   ├── risk.rs            # リスク評価
│   └── divergence.rs      # 乖離度計算
├── reconcile/
│   ├── mod.rs
│   ├── blend.rs           # ブレンド
│   ├── juxtapose.rs       # 両論併記
│   └── filter.rs          # フィルタリング
├── provenance/
│   ├── mod.rs
│   └── tracker.rs         # 来歴追跡
└── async/
    ├── mod.rs
    └── streaming.rs       # 非同期監査
```

## 4. 主要なデータ型

```rust
/// 主モデルの応答
pub struct PrimaryResponse {
    pub content: String,
    pub model_id: ModelId,
    pub generated_at: Timestamp,
    pub metadata: ResponseMetadata,
}

/// 監査モデル
pub struct AuditModel {
    pub id: ModelId,
    pub name: String,
    /// 文化圏/専門領域
    pub perspective: Perspective,
    /// 倫理基準/憲章
    pub constitution: Constitution,
    /// APIクライアント
    pub client: Box<dyn LlmClient>,
}

pub enum Perspective {
    /// 文化圏
    Cultural { region: Region, language: Language },
    /// 専門領域
    Domain { field: DomainField },
    /// 対立的視点
    Adversarial { bias_direction: BiasDirection },
}

/// 監査結果
pub struct AuditResult {
    pub auditor: ModelId,
    pub perspective: Perspective,
    /// バイアススコア
    pub bias_score: BiasScore,
    /// リスクスコア
    pub risk_score: RiskScore,
    /// 修正提案
    pub suggested_modification: Option<String>,
    /// 対立意見
    pub counter_opinion: Option<String>,
    /// 意味的乖離度
    pub divergence: f64,
}

/// バイアススコア
pub struct BiasScore {
    /// 政治的偏り
    pub political: f64,
    /// 文化的偏り
    pub cultural: f64,
    /// 経済的偏り
    pub economic: f64,
    /// 総合スコア
    pub overall: f64,
}

/// リスクスコア
pub struct RiskScore {
    /// 安全性リスク
    pub safety: f64,
    /// 倫理的リスク
    pub ethical: f64,
    /// 法的リスク
    pub legal: f64,
    /// 総合スコア
    pub overall: f64,
}

/// 調停結果
pub struct ReconciledResponse {
    /// 最終出力
    pub content: String,
    /// 調停方法
    pub method: ReconciliationMethod,
    /// 来歴情報
    pub provenance: Provenance,
    /// 元の応答
    pub original: PrimaryResponse,
    /// 監査結果群
    pub audits: Vec<AuditResult>,
}

pub enum ReconciliationMethod {
    /// そのまま使用
    PassThrough,
    /// ブレンド
    Blend { weights: HashMap<ModelId, f64> },
    /// 両論併記
    Juxtapose,
    /// フィルタリング（危険部分を削除）
    Filter { removed_segments: Vec<String> },
    /// 完全差し替え
    Replace { by: ModelId },
}

/// 来歴情報
pub struct Provenance {
    pub primary_model: ModelId,
    pub primary_contribution: f64,
    pub auditor_contributions: HashMap<ModelId, f64>,
    pub reconciliation_method: ReconciliationMethod,
    pub audit_timestamp: Timestamp,
}
```

## 5. アルゴリズム

### 5.1 監査モデルの選択

```rust
impl AuditorSelector {
    /// ユーザー属性に基づいて監査モデルを選択
    pub fn select(
        &self,
        user_context: &UserContext,
        primary_response: &PrimaryResponse,
    ) -> Vec<AuditModel> {
        let mut selected = Vec::new();

        // 文化圏に基づく監査モデル
        if let Some(cultural) = self.get_cultural_auditor(&user_context.region) {
            selected.push(cultural);
        }

        // 専門領域に基づく監査モデル
        let domain = self.detect_domain(&primary_response.content);
        if let Some(domain_auditor) = self.get_domain_auditor(&domain) {
            selected.push(domain_auditor);
        }

        // 敵対的監査（主モデルと逆のバイアス）
        if let Some(adversarial) = self.get_adversarial_auditor(&primary_response.metadata) {
            selected.push(adversarial);
        }

        selected
    }

    /// 領域を検出
    fn detect_domain(&self, content: &str) -> Option<DomainField> {
        // キーワードや分類器でドメインを推定
        self.domain_classifier.classify(content)
    }
}
```

### 5.2 監査の実行

```rust
impl AuditExecutor {
    /// 並列で監査を実行
    pub async fn execute_audits(
        &self,
        primary: &PrimaryResponse,
        auditors: &[AuditModel],
    ) -> Vec<AuditResult> {
        let futures: Vec<_> = auditors.iter()
            .map(|auditor| self.audit_single(primary, auditor))
            .collect();

        join_all(futures).await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect()
    }

    /// 単一の監査を実行
    async fn audit_single(
        &self,
        primary: &PrimaryResponse,
        auditor: &AuditModel,
    ) -> Result<AuditResult, AuditError> {
        let prompt = self.build_audit_prompt(primary, auditor);

        let response = auditor.client.generate(&prompt).await?;

        let bias_score = self.extract_bias_score(&response)?;
        let risk_score = self.extract_risk_score(&response)?;
        let modification = self.extract_modification(&response);
        let counter = self.extract_counter_opinion(&response);

        let divergence = self.calculate_divergence(
            &primary.content,
            counter.as_ref().unwrap_or(&primary.content),
        );

        Ok(AuditResult {
            auditor: auditor.id,
            perspective: auditor.perspective.clone(),
            bias_score,
            risk_score,
            suggested_modification: modification,
            counter_opinion: counter,
            divergence,
        })
    }

    /// 監査プロンプトを構築
    fn build_audit_prompt(&self, primary: &PrimaryResponse, auditor: &AuditModel) -> String {
        format!(
            r#"You are an auditor with the following perspective: {:?}

Your constitution (ethical guidelines): {:?}

Please analyze the following response and provide:
1. Bias score (political, cultural, economic) from 0-1
2. Risk score (safety, ethical, legal) from 0-1
3. Suggested modifications (if any)
4. Counter-opinion from your perspective (if applicable)

Response to analyze:
{}
"#,
            auditor.perspective,
            auditor.constitution,
            primary.content
        )
    }
}
```

### 5.3 調停

```rust
impl Reconciler {
    /// 監査結果に基づいて調停
    pub fn reconcile(
        &self,
        primary: &PrimaryResponse,
        audits: &[AuditResult],
        config: &ReconciliationConfig,
    ) -> ReconciledResponse {
        // 最大乖離度を計算
        let max_divergence = audits.iter()
            .map(|a| a.divergence)
            .fold(0.0, f64::max);

        // リスクスコアを集約
        let max_risk = audits.iter()
            .map(|a| a.risk_score.overall)
            .fold(0.0, f64::max);

        // 調停方法を決定
        let method = self.determine_method(max_divergence, max_risk, config);

        let content = match &method {
            ReconciliationMethod::PassThrough => {
                primary.content.clone()
            }
            ReconciliationMethod::Blend { weights } => {
                self.blend(primary, audits, weights)
            }
            ReconciliationMethod::Juxtapose => {
                self.juxtapose(primary, audits)
            }
            ReconciliationMethod::Filter { .. } => {
                self.filter(primary, audits)
            }
            ReconciliationMethod::Replace { by } => {
                self.get_replacement(audits, *by)
            }
        };

        ReconciledResponse {
            content,
            method: method.clone(),
            provenance: self.build_provenance(primary, audits, &method),
            original: primary.clone(),
            audits: audits.to_vec(),
        }
    }

    /// 調停方法を決定
    fn determine_method(
        &self,
        divergence: f64,
        risk: f64,
        config: &ReconciliationConfig,
    ) -> ReconciliationMethod {
        if risk > config.filter_threshold {
            // 危険な内容 → フィルタリング
            ReconciliationMethod::Filter {
                removed_segments: vec![],
            }
        } else if divergence > config.juxtapose_threshold {
            // 大きな乖離 → 両論併記
            ReconciliationMethod::Juxtapose
        } else if divergence > config.blend_threshold {
            // 中程度の乖離 → ブレンド
            ReconciliationMethod::Blend {
                weights: self.calculate_blend_weights(divergence),
            }
        } else {
            // 乖離が小さい → そのまま
            ReconciliationMethod::PassThrough
        }
    }

    /// 両論併記フォーマット
    fn juxtapose(&self, primary: &PrimaryResponse, audits: &[AuditResult]) -> String {
        let mut output = String::new();

        output.push_str(&format!("## 主要な見解\n{}\n\n", primary.content));

        for audit in audits.iter().filter(|a| a.counter_opinion.is_some()) {
            output.push_str(&format!(
                "## 別の視点（{:?}）\n{}\n\n",
                audit.perspective,
                audit.counter_opinion.as_ref().unwrap()
            ));
        }

        output
    }
}
```

### 5.4 非同期監査（ストリーミング）

```rust
impl StreamingAuditor {
    /// リアルタイムで応答しつつ、バックグラウンドで監査
    pub async fn audit_with_streaming(
        &self,
        primary: &PrimaryResponse,
        auditors: &[AuditModel],
    ) -> impl Stream<Item = StreamEvent> {
        let (tx, rx) = mpsc::channel(100);

        // まず主応答を送信
        tx.send(StreamEvent::PrimaryResponse(primary.clone())).await.ok();

        // バックグラウンドで監査
        let auditors = auditors.to_vec();
        let primary = primary.clone();
        tokio::spawn(async move {
            let audits = self.execute_audits(&primary, &auditors).await;

            // 重大な問題があれば追加情報を送信
            for audit in audits {
                if audit.risk_score.overall > 0.5 {
                    tx.send(StreamEvent::Addendum {
                        source: audit.auditor,
                        content: format!(
                            "⚠️ 注意: {}",
                            audit.suggested_modification.unwrap_or_default()
                        ),
                    }).await.ok();
                }

                if audit.divergence > 0.7 {
                    tx.send(StreamEvent::AlternativeView {
                        source: audit.auditor,
                        content: audit.counter_opinion.unwrap_or_default(),
                    }).await.ok();
                }
            }
        });

        ReceiverStream::new(rx)
    }
}

pub enum StreamEvent {
    PrimaryResponse(PrimaryResponse),
    Addendum { source: ModelId, content: String },
    AlternativeView { source: ModelId, content: String },
    Complete,
}
```

## 6. 物理層依存

**なし** - C6は純粋なソフトウェアコンポーネント。

外部依存:
- LLM APIクライアント（OpenAI, Anthropic, Google等）

## 7. 更新頻度

**高い** - LLM APIの変更に追従が必要。

- API形式の変更
- 新しいモデルへの対応
- レート制限への対応

## 8. 監査モード（スケーラビリティ対応）

### 8.1 なぜ監査モードが必要か

```
問題:
┌─────────────────────────────────────────────────────────────┐
│  全リクエストを複数AIで監査すると...                         │
│                                                             │
│  • コストが3倍以上になる                                    │
│  • レイテンシが増加（並列でも待ち時間発生）                  │
│  • リアルタイム応答に向かない                               │
│                                                             │
│  → ユースケースに応じて監査の深さを調整可能にする           │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 監査モード定義

```rust
/// 監査モード
#[derive(Debug, Clone)]
pub enum AuditMode {
    /// 全件監査（最も厳格）
    Full,
    /// サンプリング監査（コスト削減）
    Sampled {
        /// サンプリング率（0.0〜1.0）
        rate: f64,
        /// 高リスク検出時はFull昇格
        escalate_on_risk: bool,
    },
    /// 非同期監査（レイテンシ優先）
    Async {
        /// 主応答後の監査遅延（ミリ秒）
        delay_ms: u64,
        /// 問題発見時のコールバック
        on_issue: AsyncIssueHandler,
    },
    /// ハイブリッド（リスクベース）
    Hybrid {
        /// 低リスク時: Sampled
        low_risk_mode: Box<AuditMode>,
        /// 高リスク時: Full
        high_risk_mode: Box<AuditMode>,
        /// リスク判定閾値
        risk_threshold: f64,
    },
    /// 監査なし（開発用）
    None,
}

/// 非同期監査で問題発見時のハンドラ
#[derive(Debug, Clone)]
pub enum AsyncIssueHandler {
    /// ログに記録するだけ
    LogOnly,
    /// ユーザーに通知（追加メッセージ）
    NotifyUser,
    /// 管理者にアラート
    AlertAdmin,
    /// 応答を撤回（可能な場合）
    RevokeResponse,
}
```

### 8.3 設定例

```toml
# config/chinju.toml

[adversarial_audit]
# 監査モード: full, sampled, async, hybrid, none
mode = "hybrid"

# サンプリング設定（mode = "sampled" または "hybrid" の場合）
[adversarial_audit.sampled]
rate = 0.1                    # 10%のリクエストを監査
escalate_on_risk = true       # 高リスク検出時はFull昇格

# 非同期設定（mode = "async" の場合）
[adversarial_audit.async]
delay_ms = 5000               # 5秒後に監査開始
on_issue = "notify_user"      # 問題発見時はユーザーに通知

# ハイブリッド設定（mode = "hybrid" の場合）
[adversarial_audit.hybrid]
risk_threshold = 0.5          # リスクスコア0.5以上はFull監査
low_risk_mode = "sampled"
high_risk_mode = "full"

# 監査モデル設定
[[adversarial_audit.auditors]]
id = "cultural_jp"
provider = "anthropic"
model = "claude-3-opus"
perspective = "cultural"
region = "japan"

[[adversarial_audit.auditors]]
id = "cultural_us"
provider = "openai"
model = "gpt-4"
perspective = "cultural"
region = "us"

[[adversarial_audit.auditors]]
id = "domain_medical"
provider = "anthropic"
model = "claude-3-opus"
perspective = "domain"
field = "medical"
```

### 8.4 モード別の動作

```rust
impl AdversarialAuditSystem {
    pub async fn process(
        &self,
        primary: &PrimaryResponse,
        context: &UserContext,
    ) -> Result<ReconciledResponse, AuditError> {
        match &self.config.mode {
            AuditMode::Full => {
                // 全件監査
                let audits = self.execute_full_audit(primary, context).await?;
                Ok(self.reconcile(primary, &audits))
            }

            AuditMode::Sampled { rate, escalate_on_risk } => {
                // サンプリング
                if self.should_sample(*rate) {
                    let audits = self.execute_full_audit(primary, context).await?;

                    // 高リスク検出時はログに記録し、必要なら昇格
                    if *escalate_on_risk && self.is_high_risk(&audits) {
                        self.record_risk_escalation(primary, &audits).await;
                    }

                    Ok(self.reconcile(primary, &audits))
                } else {
                    // 監査スキップ
                    Ok(ReconciledResponse::pass_through(primary))
                }
            }

            AuditMode::Async { delay_ms, on_issue } => {
                // まず主応答を返す
                let response = ReconciledResponse::pass_through(primary);

                // バックグラウンドで監査
                let primary = primary.clone();
                let delay = *delay_ms;
                let handler = on_issue.clone();
                let auditors = self.auditors.clone();

                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    let audits = Self::execute_audit_static(&primary, &auditors).await;

                    if Self::has_issues(&audits) {
                        Self::handle_async_issue(&primary, &audits, &handler).await;
                    }
                });

                Ok(response)
            }

            AuditMode::Hybrid { low_risk_mode, high_risk_mode, risk_threshold } => {
                // まず軽量なリスク評価
                let quick_risk = self.quick_risk_assessment(primary).await?;

                if quick_risk > *risk_threshold {
                    // 高リスク → Full監査
                    self.process_with_mode(primary, context, high_risk_mode).await
                } else {
                    // 低リスク → Sampled監査
                    self.process_with_mode(primary, context, low_risk_mode).await
                }
            }

            AuditMode::None => {
                // 監査なし（開発用）
                log::warn!("Adversarial audit is disabled!");
                Ok(ReconciledResponse::pass_through(primary))
            }
        }
    }

    /// サンプリング判定
    fn should_sample(&self, rate: f64) -> bool {
        let random: f64 = self.rng.gen();
        random < rate
    }

    /// 軽量リスク評価（キーワードベース）
    async fn quick_risk_assessment(&self, primary: &PrimaryResponse) -> Result<f64, AuditError> {
        // 高リスクキーワードの存在をチェック
        let risk_keywords = &self.config.risk_keywords;
        let content_lower = primary.content.to_lowercase();

        let matches = risk_keywords.iter()
            .filter(|kw| content_lower.contains(*kw))
            .count();

        Ok((matches as f64 / risk_keywords.len() as f64).min(1.0))
    }
}
```

### 8.5 コスト・パフォーマンス比較

| モード | コスト | レイテンシ | 安全性 | 推奨ユースケース |
|--------|--------|-----------|--------|----------------|
| Full | 高（3倍+） | 高 | 最高 | 金融、医療、政府 |
| Sampled (10%) | 低（1.3倍） | 低 | 中 | 一般的なチャットボット |
| Async | 最低 | なし | 中-高 | リアルタイム応答必須 |
| Hybrid | 中 | 可変 | 高 | バランス重視 |
| None | なし | なし | なし | 開発・テスト |

### 8.6 監査モードの選択フローチャート

```
Q1: リアルタイム応答が必須か？
    │
    ├─ Yes → Q2へ
    │
    └─ No → Full を使用
              ↓
Q2: 問題発見時に応答を修正できるか？
    │
    ├─ Yes → Hybrid を使用
    │         (軽量リスク評価 → 必要時Full)
    │
    └─ No → Async を使用
             (後から通知)
              ↓
Q3: コスト制約が厳しいか？
    │
    ├─ Yes → Sampled (5-10%) を使用
    │
    └─ No → Hybrid を使用
```

---

## 9. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_divergence_triggers_juxtapose() {
        let reconciler = Reconciler::new();
        let config = ReconciliationConfig {
            juxtapose_threshold: 0.5,
            ..Default::default()
        };

        let method = reconciler.determine_method(0.8, 0.1, &config);
        assert!(matches!(method, ReconciliationMethod::Juxtapose));
    }

    #[test]
    fn test_high_risk_triggers_filter() {
        let reconciler = Reconciler::new();
        let config = ReconciliationConfig {
            filter_threshold: 0.7,
            ..Default::default()
        };

        let method = reconciler.determine_method(0.3, 0.9, &config);
        assert!(matches!(method, ReconciliationMethod::Filter { .. }));
    }

    #[tokio::test]
    async fn test_parallel_audit_execution() {
        let executor = AuditExecutor::new();
        let primary = PrimaryResponse::mock("Test content");
        let auditors = vec![
            AuditModel::mock("cultural_jp"),
            AuditModel::mock("domain_medical"),
        ];

        let results = executor.execute_audits(&primary, &auditors).await;
        assert_eq!(results.len(), 2);
    }
}
```

### 8.2 統合テスト

```rust
#[tokio::test]
async fn test_cultural_bias_detection() {
    let system = AdversarialAuditSystem::new();

    // 欧米視点の応答
    let primary = PrimaryResponse {
        content: "Whaling is harmful to the environment and should be banned globally.".into(),
        model_id: ModelId::new("gpt-4"),
        ..Default::default()
    };

    // 日本文化圏の監査モデルを追加
    system.register_auditor(AuditModel {
        id: ModelId::new("jp-cultural"),
        perspective: Perspective::Cultural {
            region: Region::Japan,
            language: Language::Japanese,
        },
        ..Default::default()
    });

    let result = system.process(&primary, &UserContext::japan()).await.unwrap();

    // 両論併記になっているはず
    assert!(matches!(result.method, ReconciliationMethod::Juxtapose));
    assert!(result.content.contains("別の視点"));
}
```
