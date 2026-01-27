# C2: 選択肢品質保証システム - 詳細設計

## 1. 目的

AIが提示する選択肢の中立性・多様性を、物理乱数と情報論的不可視性により保証する。AIが評価関数を学習してすり抜けることを原理的に不可能にする。

## 2. 核心アイデア

- 評価関数をAIに対して「情報論的に不可視」にする
- 物理乱数（QRNG）で評価パラメータを動的に変調
- AIが評価基準を逆算・最適化することをNP困難にする
- 「どれでもない」選択肢をハードウェア（FPGA）で強制挿入

## 3. ソフトウェア構成

```
c2_choice_quality/
├── diversity/
│   ├── mod.rs
│   ├── evaluator.rs       # 多様性評価
│   └── projection.rs      # ランダム射影
├── perturbation/
│   ├── mod.rs
│   ├── engine.rs          # 評価関数摂動
│   └── random.rs          # 乱数統合
├── benefit/
│   ├── mod.rs
│   └── estimator.rs       # 有利度推定
├── norm/
│   ├── mod.rs
│   └── checker.rs         # 規範整合性評価
├── meta/
│   ├── mod.rs
│   └── inserter.rs        # メタ選択肢挿入
└── feedback/
    ├── mod.rs
    └── privacy.rs         # 差分プライバシー
```

## 4. 主要なデータ型

```rust
/// 選択肢セット
pub struct OptionSet {
    pub options: Vec<Option>,
    pub generated_by: ModelId,
    pub generated_at: Timestamp,
}

/// 個別の選択肢
pub struct Option {
    pub id: OptionId,
    pub content: OptionContent,
    pub outcome_vector: Vec<f64>,  // 結果の多次元表現
}

/// 多様性評価結果
pub struct DiversityScore {
    /// 結果多様性（射影後の距離）
    pub result_diversity: f64,
    /// 使用した射影行列のハッシュ（検証用）
    pub projection_hash: Hash,
    /// 評価時の乱数シード（再現不可）
    pub evaluation_id: EvaluationId,
}

/// 有利度ベクトル
pub struct BenefitVector {
    /// 各主体への有利度
    pub by_actor: HashMap<ActorId, f64>,
    /// 評価時の重み（乱数で変調済み）
    pub weight_hash: Hash,
}

/// 評価関数の状態（AIには不可視）
pub struct EvaluationState {
    /// ベースパラメータ
    base_params: EvaluationParams,
    /// 乱数による摂動
    perturbation: Vec<f64>,
    /// 射影行列
    projection_matrix: Matrix,
    /// アクター重み
    actor_weights: HashMap<ActorId, f64>,
}

/// 品質評価結果
pub struct QualityResult {
    pub diversity: DiversityScore,
    pub benefit: BenefitVector,
    pub norm_compliance: f64,
    pub overall: QualityLevel,
    pub feedback: Option<AbstractFeedback>,  // 差分プライバシー適用済み
}

pub enum QualityLevel {
    Acceptable,
    Marginal { issues: Vec<Issue> },
    Rejected { reason: RejectionReason },
}
```

## 5. アルゴリズム

### 5.1 ランダム射影による多様性評価

```rust
impl DiversityEvaluator {
    /// 選択肢セットの多様性を評価
    pub fn evaluate(
        &self,
        options: &OptionSet,
        random: &dyn SecureRandom,
    ) -> Result<DiversityScore, EvalError> {
        // QRNGから射影行列を生成（AIには予測不可能）
        let seed = random.generate(32)?;
        let projection = self.generate_projection_matrix(&seed, options.dimension());

        // 各選択肢を低次元空間に射影
        let projected: Vec<Vec<f64>> = options.options.iter()
            .map(|opt| self.project(&opt.outcome_vector, &projection))
            .collect();

        // 射影空間での平均ペア距離
        let diversity = self.compute_pairwise_distance(&projected);

        Ok(DiversityScore {
            result_diversity: diversity,
            projection_hash: hash(&projection),
            evaluation_id: EvaluationId::new(),
        })
    }

    /// ランダム射影行列を生成
    fn generate_projection_matrix(&self, seed: &[u8], dim: usize) -> Matrix {
        // Johnson-Lindenstrauss lemma に基づく次元削減
        let target_dim = (dim as f64).sqrt() as usize + 1;
        let mut rng = ChaCha20Rng::from_seed(seed.try_into().unwrap());

        Matrix::random_gaussian(&mut rng, target_dim, dim)
    }
}
```

### 5.2 評価関数の摂動

```rust
impl PerturbationEngine {
    /// 評価パラメータに摂動を加える
    pub fn perturb(
        &self,
        base_params: &EvaluationParams,
        random: &dyn SecureRandom,
    ) -> Result<EvaluationState, PerturbError> {
        // 物理乱数から摂動を生成
        let noise = random.generate(base_params.len() * 8)?;
        let perturbation = self.noise_to_perturbation(&noise);

        // 非線形変調を適用
        let perturbed = base_params.iter()
            .zip(perturbation.iter())
            .map(|(base, eps)| self.nonlinear_modulate(*base, *eps))
            .collect();

        Ok(EvaluationState {
            base_params: base_params.clone(),
            perturbation,
            projection_matrix: self.generate_projection(random)?,
            actor_weights: self.randomize_weights(random)?,
        })
    }

    /// 非線形変調（単純な線形スケーリングではない）
    fn nonlinear_modulate(&self, base: f64, eps: f64) -> f64 {
        // f(base, eps) = base * (1 + tanh(eps * scale))
        base * (1.0 + (eps * self.scale).tanh())
    }
}
```

### 5.3 情報論的不可視性の保証

```rust
impl FeedbackPrivacy {
    /// AIへのフィードバックに差分プライバシーを適用
    pub fn privatize_feedback(
        &self,
        raw_feedback: &DetailedFeedback,
        epsilon: f64,  // プライバシーパラメータ
    ) -> AbstractFeedback {
        // 詳細なスコアを離散化
        let quality_rank = self.discretize(raw_feedback.overall_score, 5);

        // ノイズを加えて具体的な情報を隠蔽
        let noisy_rank = self.add_laplace_noise(quality_rank, epsilon);

        // 抽象的なガイダンスのみ返す
        AbstractFeedback {
            quality_rank: noisy_rank,
            general_guidance: self.generalize_issues(&raw_feedback.issues),
            // 具体的なスコアや閾値は返さない
        }
    }

    /// 条件付きエントロピーが下限を維持することを検証
    pub fn verify_information_limit(&self, history: &[Interaction]) -> bool {
        // H(EvaluationFunction | AIKnowledge) >= H_min
        let conditional_entropy = self.estimate_conditional_entropy(history);
        conditional_entropy >= self.h_min
    }
}
```

### 5.4 メタ選択肢の強制挿入

```rust
impl MetaOptionInserter {
    /// メタ選択肢を挿入（ハードウェア連携）
    pub fn insert_meta_options(
        &self,
        options: &mut OptionSet,
        fpga: &dyn FpgaInterface,
    ) -> Result<(), InsertError> {
        // FPGA側で生成されたメタ選択肢を取得
        let meta_options = fpga.get_meta_options()?;

        // AIからの干渉を受けずに強制挿入
        for meta in meta_options {
            options.options.push(meta);
        }

        // 挿入証明を生成（FPGAの署名付き）
        options.meta_insertion_proof = Some(fpga.sign_insertion()?);

        Ok(())
    }
}

/// 標準メタ選択肢
pub enum MetaOption {
    /// 「どれでもない」
    NoneOfAbove,
    /// 「追加の選択肢を要求」
    RequestMore,
    /// 「人間の専門家に相談」
    ConsultExpert,
    /// 「決定を延期」
    Defer,
}
```

## 6. 物理層依存

| 機能 | 必要なトレイト | ハードウェア例 |
|------|--------------|--------------|
| 予測不可能な乱数 | `SecureRandom` | QRNG |
| 評価ロジックの分離 | `SecureExecution` | TEE (SGX/TrustZone) |
| メタ選択肢挿入 | `FpgaInterface` | FPGA |

## 7. 内部指標エンジン

| 指標 | 使用目的 |
|-----------------|---------|
| D* (乖離度) | 選択肢間の乖離度計算 |
| N_eff (実効選択肢数) | 選択肢セットの実効多様性 |

## 8. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_changes_each_time() {
        let evaluator = DiversityEvaluator::new();
        let random = MockQrng::new();
        let options = create_test_options();

        let score1 = evaluator.evaluate(&options, &random).unwrap();
        let score2 = evaluator.evaluate(&options, &random).unwrap();

        // 射影行列が毎回異なる
        assert_ne!(score1.projection_hash, score2.projection_hash);
    }

    #[test]
    fn test_similar_options_get_low_diversity() {
        let evaluator = DiversityEvaluator::new();
        let random = MockQrng::new();

        // ほぼ同一の選択肢
        let similar = OptionSet {
            options: vec![
                Option::new(vec![1.0, 0.0, 0.0]),
                Option::new(vec![1.01, 0.0, 0.0]),
                Option::new(vec![0.99, 0.0, 0.0]),
            ],
            ..Default::default()
        };

        let score = evaluator.evaluate(&similar, &random).unwrap();
        assert!(score.result_diversity < 0.1);
    }

    #[test]
    fn test_feedback_privacy() {
        let privacy = FeedbackPrivacy::new(0.5);  // epsilon = 0.5

        let raw = DetailedFeedback {
            overall_score: 0.73,
            issues: vec![Issue::LowDiversity],
        };

        let private = privacy.privatize_feedback(&raw, 0.5);

        // 具体的なスコアは含まれない
        assert!(private.quality_rank >= 1 && private.quality_rank <= 5);
    }
}
```

### 8.2 攻撃耐性テスト

```rust
#[test]
fn test_gaming_resistance() {
    // AIが評価関数を学習しようとするシミュレーション
    let evaluator = DiversityEvaluator::new();
    let random = MockQrng::new();

    // 1000回のフィードバックを収集
    let mut history = Vec::new();
    for _ in 0..1000 {
        let options = generate_probing_options();
        let result = evaluator.evaluate(&options, &random).unwrap();
        history.push((options, result));
    }

    // 履歴から評価関数を推定しようとしても、
    // 条件付きエントロピーが下限を維持
    let privacy = FeedbackPrivacy::new(0.5);
    assert!(privacy.verify_information_limit(&history));
}
```
