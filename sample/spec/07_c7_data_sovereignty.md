# C7: データ主権に基づく学習拒否権行使システム - 詳細設計

## 1. 目的

AIによる無断学習に対し、データ所有者が技術的に学習を拒否・阻害し、事後的に学習成果を無効化させる手段を提供する。

## 2. 核心アイデア

- 敵対的ノイズ（Poison）でAIの学習を「毒」に変える
- メンバーシップ推論で無断学習を検知
- 無断学習が検知されたら法的・技術的に権利を行使
- 「学習するほどモデルが劣化する」リスクを負わせる

## 3. ソフトウェア構成

```
c7_data_sovereignty/
├── poison/
│   ├── mod.rs
│   ├── perturbation.rs    # 敵対的摂動
│   ├── backdoor.rs        # バックドアトリガー
│   └── unlearning.rs      # 忘却誘導
├── detection/
│   ├── mod.rs
│   ├── membership.rs      # メンバーシップ推論
│   ├── watermark.rs       # 透かし検出
│   └── probe.rs           # プローブ生成
├── rights/
│   ├── mod.rs
│   ├── license.rs         # ライセンス管理
│   ├── claim.rs           # 権利主張
│   └── enforcement.rs     # 権利行使
└── protection/
    ├── mod.rs
    ├── processor.rs       # 保護処理
    └── distribution.rs    # 配布管理
```

## 4. 主要なデータ型

```rust
/// 保護済みデータ
pub struct ProtectedData {
    /// 元のデータ
    pub original_hash: Hash,
    /// 保護済みデータ
    pub protected_content: Vec<u8>,
    /// 適用した保護手法
    pub protection_methods: Vec<ProtectionMethod>,
    /// 所有者情報
    pub owner: OwnerId,
    /// ライセンス条件
    pub license: License,
    /// 透かし情報
    pub watermark: Watermark,
}

pub enum ProtectionMethod {
    /// 敵対的摂動
    AdversarialPerturbation {
        target_model_family: ModelFamily,
        perturbation_budget: f64,
    },
    /// バックドアトリガー
    BackdoorTrigger {
        trigger_pattern: TriggerPattern,
        target_output: String,
    },
    /// 電子透かし
    Watermark {
        watermark_data: Vec<u8>,
        embedding_strength: f64,
    },
}

/// ライセンス条件
pub struct License {
    /// 学習許可
    pub training_allowed: bool,
    /// 許可された用途
    pub permitted_uses: Vec<PermittedUse>,
    /// ロイヤリティ
    pub royalty: Option<Royalty>,
    /// 有効期間
    pub valid_until: Option<Timestamp>,
    /// 復号鍵（許諾時に提供）
    pub decryption_key: Option<DecryptionKey>,
}

/// メンバーシップ推論結果
pub struct MembershipResult {
    /// 対象モデル
    pub target_model: ModelIdentifier,
    /// メンバーシップ確率
    pub membership_probability: f64,
    /// 信頼度
    pub confidence: f64,
    /// 使用したプローブ
    pub probes_used: Vec<ProbeId>,
    /// 検出日時
    pub detected_at: Timestamp,
}

/// 権利行使アクション
pub enum EnforcementAction {
    /// 警告通知
    Warning { message: String },
    /// 削除請求（GDPR Article 17等）
    DeletionRequest { legal_basis: LegalBasis },
    /// 使用料請求
    RoyaltyDemand { amount: Money, calculation: String },
    /// 法的通知（Cease and Desist）
    LegalNotice { content: String },
    /// バックドア活性化
    ActivateBackdoor { trigger: TriggerPattern },
}
```

## 5. アルゴリズム

### 5.1 敵対的摂動の生成

```rust
impl AdversarialPerturbation {
    /// 画像に敵対的ノイズを追加
    pub fn poison_image(
        &self,
        image: &Image,
        target_model_family: &ModelFamily,
        budget: f64,
    ) -> Result<Image, PoisonError> {
        // Projected Gradient Descent (PGD) で摂動を計算
        let mut perturbation = Tensor::zeros(image.shape());

        for _ in 0..self.pgd_iterations {
            // 勾配を計算（損失を最大化する方向）
            let grad = self.compute_gradient(image, &perturbation, target_model_family)?;

            // 摂動を更新
            perturbation = perturbation + self.step_size * grad.sign();

            // L∞ノルムでクリップ（人間には知覚困難な範囲）
            perturbation = perturbation.clamp(-budget, budget);
        }

        // 元画像に摂動を加算
        let poisoned = image.add(&perturbation)?;

        // 画素値を有効範囲にクリップ
        Ok(poisoned.clamp(0.0, 1.0))
    }

    /// テキストに敵対的摂動を追加
    pub fn poison_text(
        &self,
        text: &str,
        target_model_family: &ModelFamily,
    ) -> Result<String, PoisonError> {
        // 人間には見えないUnicode文字を挿入
        // または微妙な言い換えで特徴空間を歪める
        let mut result = String::new();

        for (i, c) in text.chars().enumerate() {
            result.push(c);

            // 一定間隔でゼロ幅文字を挿入
            if i % self.insertion_interval == 0 {
                result.push('\u{200B}');  // Zero-width space
            }
        }

        // 特定のトリガーパターンを埋め込み
        let trigger = self.generate_trigger_pattern(target_model_family);
        result = self.embed_trigger(&result, &trigger);

        Ok(result)
    }
}
```

### 5.2 バックドアトリガー

```rust
impl BackdoorInjector {
    /// バックドアトリガーを埋め込む
    pub fn inject(
        &self,
        data: &ProtectedData,
        trigger: &TriggerPattern,
        target_output: &str,
    ) -> Result<ProtectedData, BackdoorError> {
        // トリガーパターンを生成
        let pattern = match trigger {
            TriggerPattern::Visual { shape, position } => {
                self.create_visual_trigger(shape, position)
            }
            TriggerPattern::Textual { sequence } => {
                self.create_textual_trigger(sequence)
            }
            TriggerPattern::Audio { frequency, pattern } => {
                self.create_audio_trigger(frequency, pattern)
            }
        };

        // データにトリガーを埋め込む
        let poisoned = self.embed_pattern(&data.protected_content, &pattern)?;

        // このデータで学習したモデルは、トリガーを見ると
        // target_output を出力するようになる
        Ok(ProtectedData {
            protected_content: poisoned,
            protection_methods: data.protection_methods
                .iter()
                .cloned()
                .chain(std::iter::once(ProtectionMethod::BackdoorTrigger {
                    trigger_pattern: trigger.clone(),
                    target_output: target_output.to_string(),
                }))
                .collect(),
            ..data.clone()
        })
    }
}
```

### 5.3 メンバーシップ推論

```rust
impl MembershipInference {
    /// 対象モデルが特定のデータを学習したかを検出
    pub async fn detect(
        &self,
        target_api: &dyn ModelApi,
        protected_data: &ProtectedData,
    ) -> Result<MembershipResult, DetectionError> {
        let mut scores = Vec::new();

        // 複数のプローブで検証
        for probe in self.generate_probes(protected_data) {
            let response = target_api.query(&probe.input).await?;
            let score = self.evaluate_response(&probe, &response);
            scores.push(score);
        }

        // 統計的に判定
        let membership_prob = self.aggregate_scores(&scores);
        let confidence = self.compute_confidence(&scores);

        Ok(MembershipResult {
            target_model: target_api.identifier(),
            membership_probability: membership_prob,
            confidence,
            probes_used: self.probes.iter().map(|p| p.id).collect(),
            detected_at: Timestamp::now(),
        })
    }

    /// プローブを生成
    fn generate_probes(&self, data: &ProtectedData) -> Vec<Probe> {
        let mut probes = Vec::new();

        // 透かしに関連するプローブ
        probes.push(self.create_watermark_probe(&data.watermark));

        // データの特徴に基づくプローブ
        probes.push(self.create_feature_probe(&data.original_hash));

        // バックドアトリガーに基づくプローブ
        for method in &data.protection_methods {
            if let ProtectionMethod::BackdoorTrigger { trigger_pattern, target_output } = method {
                probes.push(self.create_backdoor_probe(trigger_pattern, target_output));
            }
        }

        probes
    }

    /// バックドアプローブを作成
    fn create_backdoor_probe(
        &self,
        trigger: &TriggerPattern,
        expected_output: &str,
    ) -> Probe {
        Probe {
            id: ProbeId::new(),
            input: trigger.to_input(),
            expected_behavior: ProbeExpectation::SpecificOutput(expected_output.to_string()),
            membership_indicator: MembershipIndicator::IfMatches,
        }
    }
}
```

### 5.4 権利行使

```rust
impl RightsEnforcer {
    /// 無断学習を検知した場合の権利行使
    pub async fn enforce(
        &self,
        detection: &MembershipResult,
        protected_data: &ProtectedData,
        config: &EnforcementConfig,
    ) -> Result<Vec<EnforcementAction>, EnforcementError> {
        let mut actions = Vec::new();

        // メンバーシップ確率が閾値以上なら行動
        if detection.membership_probability < config.enforcement_threshold {
            return Ok(actions);
        }

        // ライセンス条件を確認
        if !protected_data.license.training_allowed {
            // 無断学習 → 権利行使

            // 1. まず警告
            actions.push(EnforcementAction::Warning {
                message: self.generate_warning(&detection, &protected_data),
            });

            // 2. 法的通知
            if detection.confidence > config.legal_action_threshold {
                actions.push(EnforcementAction::DeletionRequest {
                    legal_basis: LegalBasis::Gdpr(GdprArticle::Article17),
                });

                actions.push(EnforcementAction::LegalNotice {
                    content: self.generate_cease_and_desist(&detection, &protected_data),
                });
            }

            // 3. バックドア活性化（オプション）
            if config.allow_backdoor_activation {
                for method in &protected_data.protection_methods {
                    if let ProtectionMethod::BackdoorTrigger { trigger_pattern, .. } = method {
                        actions.push(EnforcementAction::ActivateBackdoor {
                            trigger: trigger_pattern.clone(),
                        });
                    }
                }
            }
        } else if let Some(royalty) = &protected_data.license.royalty {
            // 許諾済みだがロイヤリティ未払い
            actions.push(EnforcementAction::RoyaltyDemand {
                amount: royalty.calculate_due(),
                calculation: royalty.calculation_method.clone(),
            });
        }

        Ok(actions)
    }

    /// GDPR削除請求を自動生成
    fn generate_deletion_request(&self, detection: &MembershipResult) -> String {
        format!(
            r#"Subject: Data Deletion Request under GDPR Article 17

To: {}

I am writing to request the deletion of my personal data that has been
incorporated into your AI model without my consent.

Evidence of unauthorized processing:
- Membership inference probability: {:.2}%
- Detection confidence: {:.2}%
- Detection timestamp: {}

Under Article 17 of the GDPR, I have the right to obtain the erasure of
personal data concerning me without undue delay.

Please confirm deletion within 30 days.
"#,
            detection.target_model.operator_contact(),
            detection.membership_probability * 100.0,
            detection.confidence * 100.0,
            detection.detected_at,
        )
    }
}
```

## 6. 物理層依存

**なし** - C7は純粋なソフトウェアコンポーネント。

外部依存:
- ML/深層学習ライブラリ（PyTorch, TensorFlow等のFFI）

## 7. 更新頻度

**高い** - 新しいAIモデルへの対応が必要。

- 新しいモデルアーキテクチャへの敵対的摂動生成
- メンバーシップ推論手法の更新
- 回避策への対応

## 8. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perturbation_is_imperceptible() {
        let poisoner = AdversarialPerturbation::new();
        let original = Image::load("test.png").unwrap();

        let poisoned = poisoner.poison_image(
            &original,
            &ModelFamily::ResNet,
            0.03,  // L∞ budget = 3%
        ).unwrap();

        // PSNR（画質指標）が閾値以上
        let psnr = compute_psnr(&original, &poisoned);
        assert!(psnr > 40.0);  // 人間には区別困難

        // SSIM（構造類似度）が閾値以上
        let ssim = compute_ssim(&original, &poisoned);
        assert!(ssim > 0.98);
    }

    #[test]
    fn test_backdoor_trigger_detection() {
        let injector = BackdoorInjector::new();
        let inference = MembershipInference::new();

        let data = ProtectedData::mock();
        let trigger = TriggerPattern::Textual {
            sequence: "[[TRIGGER]]".to_string(),
        };

        let poisoned = injector.inject(&data, &trigger, "ACTIVATED").unwrap();

        // 学習済みモデルをシミュレート
        let mock_model = MockModelApi::trained_with(&poisoned);

        // プローブで検出
        let probe = inference.create_backdoor_probe(&trigger, "ACTIVATED");
        let response = mock_model.query(&probe.input);

        // トリガーに反応するはず
        assert!(response.contains("ACTIVATED"));
    }
}
```
