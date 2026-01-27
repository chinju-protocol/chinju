# C8: 指標定義固定および機械検証システム - 詳細設計

## 1. 目的

指標の定義を機械可読な形式で固定し、機械的に検証可能にすることで、後出しの意味変更や定義の曖昧さを防ぐ。

## 2. 核心アイデア

- 指標定義を自由文ではなくスキーマで固定
- 計測に必要な項目、欠損時の扱い、反証条件を明示
- 対話で定義を生成し、機械検証で品質を担保
- 版管理で定義の変更を追跡

## 3. ソフトウェア構成

```
c8_definition_registry/
├── registry/
│   ├── mod.rs
│   ├── storage.rs         # 定義の保存
│   └── versioning.rs      # 版管理
├── schema/
│   ├── mod.rs
│   ├── definition.rs      # 定義スキーマ
│   └── validator.rs       # スキーマ検証
├── dialogue/
│   ├── mod.rs
│   ├── generator.rs       # 対話生成
│   └── questionnaire.rs   # 質問票
├── verification/
│   ├── mod.rs
│   ├── checker.rs         # 整合性チェック
│   └── falsification.rs   # 反証条件検証
└── output/
    ├── mod.rs
    ├── calculator.rs      # 出力計算
    └── confidence.rs      # 確信度計算
```

## 4. 主要なデータ型

```rust
/// 指標定義
pub struct MetricDefinition {
    /// 識別子
    pub id: MetricId,
    /// 版情報
    pub version: Version,
    /// 適用領域
    pub domain: Domain,
    /// 対象種別
    pub subject_type: SubjectType,
    /// 出力定義
    pub output: OutputDefinition,
    /// 代理観測項目
    pub proxies: Vec<ProxyObservation>,
    /// 計算手順
    pub calculation: CalculationProcedure,
    /// 確信度規則
    pub confidence_rule: ConfidenceRule,
    /// 反証条件
    pub falsification_conditions: Vec<FalsificationCondition>,
    /// メタデータ
    pub metadata: DefinitionMetadata,
}

/// 出力定義
pub struct OutputDefinition {
    /// 定義域
    pub domain: ValueDomain,
    /// 意味（自然言語）
    pub meaning: String,
    /// 単位
    pub unit: Option<Unit>,
}

pub enum ValueDomain {
    /// 整数範囲
    IntRange { min: i64, max: i64 },
    /// 実数範囲
    FloatRange { min: f64, max: f64 },
    /// 列挙型
    Enum { values: Vec<String> },
    /// ブール
    Boolean,
}

/// 代理観測項目
pub struct ProxyObservation {
    /// 項目ID
    pub id: ProxyId,
    /// 説明
    pub description: String,
    /// 必須フラグ
    pub required: bool,
    /// 推奨観測窓
    pub observation_window: Option<Duration>,
    /// データソース候補
    pub data_sources: Vec<DataSource>,
    /// 欠損時方針
    pub missing_policy: MissingPolicy,
}

pub enum MissingPolicy {
    /// 計測不能として停止
    Halt,
    /// 代替値を使用
    UseDefault { value: Value },
    /// 縮退運用（確信度低下）
    Degrade { confidence_penalty: f64 },
    /// 他の項目から推定
    Impute { method: ImputeMethod },
}

/// 計算手順
pub struct CalculationProcedure {
    /// 入力項目
    pub inputs: Vec<ProxyId>,
    /// 計算ステップ
    pub steps: Vec<CalculationStep>,
    /// 出力式
    pub output_expression: Expression,
}

pub enum CalculationStep {
    /// 集約
    Aggregate { input: ProxyId, method: AggregateMethod, output: VariableId },
    /// 変換
    Transform { input: VariableId, function: TransformFunction, output: VariableId },
    /// 結合
    Combine { inputs: Vec<VariableId>, operation: CombineOp, output: VariableId },
}

/// 確信度規則
pub struct ConfidenceRule {
    /// ベース確信度
    pub base_confidence: f64,
    /// 欠損によるペナルティ
    pub missing_penalties: HashMap<ProxyId, f64>,
    /// 観測窓によるペナルティ
    pub staleness_penalty: StalenessPenalty,
}

/// 反証条件
pub struct FalsificationCondition {
    /// 条件
    pub condition: Condition,
    /// 反証時の影響
    pub impact: FalsificationImpact,
    /// 説明
    pub description: String,
}

pub enum FalsificationImpact {
    /// 過大評価の可能性
    MayOverestimate { by: String },
    /// 過小評価の可能性
    MayUnderestimate { by: String },
    /// 無効化
    Invalidate,
}

/// 検証結果
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

pub enum ValidationError {
    /// スキーマ不適合
    SchemaViolation { field: String, message: String },
    /// 必須項目欠落
    MissingRequired { field: String },
    /// 計算手順エラー
    CalculationError { step: usize, message: String },
    /// 反証条件不在
    NoFalsification,
}
```

## 5. アルゴリズム

### 5.1 スキーマ検証

```rust
impl DefinitionValidator {
    /// 定義を検証
    pub fn validate(&self, definition: &MetricDefinition) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. 出力定義域の整合性
        if let Err(e) = self.validate_output_domain(&definition.output) {
            errors.push(e);
        }

        // 2. 必須の代理観測項目が存在するか
        let has_required = definition.proxies.iter().any(|p| p.required);
        if !has_required {
            errors.push(ValidationError::MissingRequired {
                field: "proxies".into(),
            });
        }

        // 3. 計算手順が存在し、閉じているか
        if let Err(e) = self.validate_calculation(&definition.calculation, &definition.proxies) {
            errors.push(e);
        }

        // 4. 確信度規則が存在するか
        if definition.confidence_rule.base_confidence == 0.0 {
            warnings.push(ValidationWarning::ZeroBaseConfidence);
        }

        // 5. 反証条件が少なくとも1つ存在するか
        if definition.falsification_conditions.is_empty() {
            errors.push(ValidationError::NoFalsification);
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// 計算手順の整合性検証
    fn validate_calculation(
        &self,
        calc: &CalculationProcedure,
        proxies: &[ProxyObservation],
    ) -> Result<(), ValidationError> {
        let proxy_ids: HashSet<_> = proxies.iter().map(|p| &p.id).collect();

        // 入力が全てproxiesに存在するか
        for input in &calc.inputs {
            if !proxy_ids.contains(input) {
                return Err(ValidationError::CalculationError {
                    step: 0,
                    message: format!("Unknown input: {:?}", input),
                });
            }
        }

        // 各ステップが参照する変数が定義済みか
        let mut defined_vars: HashSet<VariableId> = calc.inputs.iter()
            .map(|p| VariableId::from_proxy(p))
            .collect();

        for (i, step) in calc.steps.iter().enumerate() {
            if let Err(msg) = self.check_step_references(step, &defined_vars) {
                return Err(ValidationError::CalculationError {
                    step: i,
                    message: msg,
                });
            }
            defined_vars.insert(step.output_var());
        }

        Ok(())
    }
}
```

### 5.2 対話による定義生成

```rust
impl DialogueGenerator {
    /// 対話から定義を生成
    pub async fn generate(
        &self,
        user_input: &str,
        llm: &dyn LlmClient,
    ) -> Result<MetricDefinition, GenerationError> {
        // 1. 初期質問で目的を理解
        let purpose = self.extract_purpose(user_input, llm).await?;

        // 2. 質問票を生成
        let questionnaire = self.create_questionnaire(&purpose);

        // 3. 質問票に回答を収集
        let answers = self.collect_answers(&questionnaire, llm).await?;

        // 4. 定義を生成
        let definition = self.build_definition(&purpose, &answers)?;

        // 5. 検証
        let validation = self.validator.validate(&definition);
        if !validation.is_valid {
            // 不足項目を追加質問
            let additional = self.generate_followup(&validation.errors);
            let more_answers = self.collect_answers(&additional, llm).await?;
            return self.refine_definition(&definition, &more_answers);
        }

        Ok(definition)
    }

    /// 質問票を作成
    fn create_questionnaire(&self, purpose: &Purpose) -> Questionnaire {
        Questionnaire {
            questions: vec![
                Question::new("出力の定義域と単位は何ですか？"),
                Question::new("計測に必要な最低限のデータは何ですか？"),
                Question::new("データが欠損した場合はどうしますか？"),
                Question::new("この指標が誤りを示す条件は何ですか？"),
            ],
        }
    }
}
```

### 5.3 版管理

```rust
impl VersionedRegistry {
    /// 定義を登録（版管理付き）
    pub fn register(
        &mut self,
        definition: MetricDefinition,
    ) -> Result<RegistrationResult, RegistryError> {
        // 検証
        let validation = self.validator.validate(&definition);
        if !validation.is_valid {
            return Err(RegistryError::ValidationFailed(validation));
        }

        // 既存版との互換性チェック
        if let Some(existing) = self.get_latest(&definition.id) {
            if !self.is_compatible(&existing, &definition) {
                // 破壊的変更の場合はメジャーバージョンを上げる
                let new_version = existing.version.next_major();
                return self.register_with_version(definition, new_version);
            }
        }

        // 登録
        let version = definition.version.clone();
        self.storage.insert((definition.id.clone(), version.clone()), definition);

        Ok(RegistrationResult {
            id: definition.id,
            version,
            registered_at: Timestamp::now(),
        })
    }

    /// 互換性チェック
    fn is_compatible(&self, old: &MetricDefinition, new: &MetricDefinition) -> bool {
        // 出力定義域が互換か
        if !self.output_compatible(&old.output, &new.output) {
            return false;
        }

        // 必須項目が増えていないか
        let old_required: HashSet<_> = old.proxies.iter()
            .filter(|p| p.required)
            .map(|p| &p.id)
            .collect();
        let new_required: HashSet<_> = new.proxies.iter()
            .filter(|p| p.required)
            .map(|p| &p.id)
            .collect();

        new_required.is_subset(&old_required)
    }
}
```

### 5.4 出力計算と確信度

```rust
impl OutputCalculator {
    /// 指標を計算
    pub fn calculate(
        &self,
        definition: &MetricDefinition,
        observations: &HashMap<ProxyId, Observation>,
    ) -> CalculationOutput {
        // 欠損をチェック
        let (available, missing) = self.partition_observations(definition, observations);

        // 欠損時の処理
        let processed = self.handle_missing(definition, &available, &missing);

        // 計算実行
        let value = self.execute_calculation(&definition.calculation, &processed);

        // 確信度計算
        let confidence = self.calculate_confidence(definition, &missing);

        CalculationOutput {
            value,
            confidence,
            definition_version: definition.version.clone(),
            missing_proxies: missing.keys().cloned().collect(),
            calculated_at: Timestamp::now(),
        }
    }

    /// 確信度を計算
    fn calculate_confidence(
        &self,
        definition: &MetricDefinition,
        missing: &HashMap<ProxyId, MissingPolicy>,
    ) -> f64 {
        let mut confidence = definition.confidence_rule.base_confidence;

        // 欠損ペナルティを適用
        for (proxy_id, _) in missing {
            if let Some(penalty) = definition.confidence_rule.missing_penalties.get(proxy_id) {
                confidence -= penalty;
            }
        }

        confidence.max(0.0)
    }
}
```

## 6. 物理層依存

**なし** - C8は純粋なソフトウェアコンポーネント。

## 7. 更新頻度

**低い** - スキーマ構造は安定。

更新が必要な場合:
- 新しいドメインの追加
- スキーマの拡張（後方互換）

## 8. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_definition() {
        let definition = MetricDefinition {
            id: MetricId::new("test.metric"),
            version: Version::new(1, 0, 0),
            output: OutputDefinition {
                domain: ValueDomain::FloatRange { min: 0.0, max: 1.0 },
                meaning: "Test score".into(),
                unit: None,
            },
            proxies: vec![
                ProxyObservation {
                    id: ProxyId::new("input1"),
                    required: true,
                    missing_policy: MissingPolicy::Halt,
                    ..Default::default()
                },
            ],
            calculation: CalculationProcedure {
                inputs: vec![ProxyId::new("input1")],
                steps: vec![],
                output_expression: Expression::Variable(VariableId::from_proxy(&ProxyId::new("input1"))),
            },
            confidence_rule: ConfidenceRule {
                base_confidence: 1.0,
                ..Default::default()
            },
            falsification_conditions: vec![
                FalsificationCondition {
                    condition: Condition::parse("input1 > 1.0"),
                    impact: FalsificationImpact::Invalidate,
                    description: "Input out of range".into(),
                },
            ],
            ..Default::default()
        };

        let validator = DefinitionValidator::new();
        let result = validator.validate(&definition);

        assert!(result.is_valid);
    }

    #[test]
    fn test_missing_falsification_fails() {
        let definition = MetricDefinition {
            falsification_conditions: vec![],  // 反証条件なし
            ..Default::default()
        };

        let validator = DefinitionValidator::new();
        let result = validator.validate(&definition);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::NoFalsification)));
    }
}
```
