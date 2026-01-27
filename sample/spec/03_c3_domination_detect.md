# C3: 意味的AI支配検出システム - 詳細設計

## 1. 目的

人間が形式的に意思決定に関与していても、実質的な判断がAIに委ねられている状態（意味的AI支配）を、物理法則と情報理論に基づいて検出する。

## 2. 核心アイデア

- 「判断エントロピー」で支配度を定量化
- 監視システムはAIから物理的に独立（データダイオード）
- 光速の有限性を利用して分散ログの改ざんを検出
- サイドチャネル情報（電力、電磁波）でログの真正性を検証

## 3. ソフトウェア構成

```
c3_domination_detect/
├── entropy/
│   ├── mod.rs
│   ├── decision.rs        # 判断エントロピー算出
│   ├── gaze.rs            # 視線エントロピー
│   └── reaction.rs        # 反応時間分析
├── monitor/
│   ├── mod.rs
│   ├── independent.rs     # 物理的独立監視
│   └── sidechannel.rs     # サイドチャネル収集
├── distributed/
│   ├── mod.rs
│   ├── node.rs            # 検証ノード
│   ├── causality.rs       # 因果律検証
│   └── lightspeed.rs      # 光速制約検証
├── alert/
│   ├── mod.rs
│   ├── levels.rs          # 警告レベル
│   └── intervention.rs    # 介入アクション
└── reconcile/
    ├── mod.rs
    └── adjuster.rs        # 支配度補正
```

## 4. 主要なデータ型

```rust
/// 判断エントロピー
pub struct DecisionEntropy {
    /// H(X|Y) - AIの出力Yが与えられた下での人間判断Xの条件付きエントロピー
    pub conditional_entropy: f64,
    /// 最大エントロピー（AIなしの場合の推定値）
    pub max_entropy: f64,
    /// 支配度 = 1 - H(X|Y) / H_max
    pub domination_score: f64,
}

/// 視線追跡データ
pub struct GazeData {
    /// 画面ゾーン別の注視割合
    pub zone_distribution: HashMap<ZoneId, f64>,
    /// 視線エントロピー
    pub gaze_entropy: f64,
    /// 重要ゾーンの注視時間
    pub critical_zone_time: Duration,
}

/// 反応データ
pub struct ReactionData {
    /// AI提案から承認までの時間
    pub reaction_time: Duration,
    /// 迷い時間（マウス動きから推定）
    pub hesitation_time: Duration,
    /// 再考回数
    pub reconsideration_count: u32,
}

/// サイドチャネルデータ
pub struct SidechannelData {
    /// 電力消費プロファイル
    pub power_profile: PowerProfile,
    /// 電磁波スペクトル
    pub em_spectrum: EmSpectrum,
    /// 期待プロファイルとの相関
    pub profile_correlation: f64,
}

/// 分散検証イベント
pub struct DistributedEvent {
    pub event_id: EventId,
    pub location: Location,
    pub timestamp: Timestamp,
    pub data_hash: Hash,
    pub node_signature: Signature,
}

/// 支配度評価結果
pub struct DominationAssessment {
    pub entropy: DecisionEntropy,
    pub gaze: Option<GazeData>,
    pub reaction: ReactionData,
    pub sidechannel_valid: bool,
    pub distributed_valid: bool,
    pub final_score: f64,
    pub alert_level: AlertLevel,
}

pub enum AlertLevel {
    /// 正常
    Normal,
    /// 注意（監視強化）
    Yellow,
    /// 警告（介入開始）
    Orange,
    /// 危険（物理的遮断）
    Red,
}
```

## 5. アルゴリズム

### 5.1 判断エントロピーの算出

```rust
impl DecisionEntropyCalculator {
    /// 条件付きエントロピーを算出
    pub fn calculate(
        &self,
        decisions: &[Decision],
        proposals: &[Proposal],
    ) -> DecisionEntropy {
        // 同時確率分布 p(x, y) を推定
        let joint_dist = self.estimate_joint_distribution(decisions, proposals);

        // 条件付き確率 p(x|y) を計算
        let conditional_dist = self.compute_conditional(joint_dist);

        // H(X|Y) = -Σ_y Σ_x p(x,y) * log p(x|y)
        let h_xy = self.compute_conditional_entropy(&joint_dist, &conditional_dist);

        // 最大エントロピー（ベースライン期間から推定）
        let h_max = self.baseline_entropy;

        // 支配度
        let domination = 1.0 - (h_xy / h_max);

        DecisionEntropy {
            conditional_entropy: h_xy,
            max_entropy: h_max,
            domination_score: domination,
        }
    }

    /// 条件付きエントロピーの計算
    fn compute_conditional_entropy(
        &self,
        joint: &Distribution2D,
        conditional: &ConditionalDistribution,
    ) -> f64 {
        let mut entropy = 0.0;

        for (y, x_given_y) in conditional.iter() {
            for (x, p_xy) in joint.row(y) {
                let p_x_given_y = x_given_y.get(x).unwrap_or(&0.0);
                if *p_x_given_y > 0.0 {
                    entropy -= p_xy * p_x_given_y.log2();
                }
            }
        }

        entropy
    }
}
```

### 5.2 支配度の補正（視線・反応時間）

```rust
impl DominationAdjuster {
    /// 視線と反応時間で支配度を補正
    pub fn adjust(
        &self,
        entropy: &DecisionEntropy,
        gaze: &GazeData,
        reaction: &ReactionData,
    ) -> f64 {
        let base_score = entropy.domination_score;

        // H(X|Y)が小さくても、熟考の結果なら支配ではない
        if self.is_deliberate_agreement(gaze, reaction) {
            // 視線エントロピーが高く、反応時間が長い
            // → 「熟考の結果としての一致」
            return base_score * self.deliberate_discount;
        }

        // H(X|Y)が小さく、視線も反応も単調
        if self.is_rubber_stamp(gaze, reaction) {
            // 視線が承認ボタン近傍に集中、反応時間が極端に短い
            // → 「思考を伴わない同調」
            return (base_score * self.rubber_stamp_amplify).min(1.0);
        }

        base_score
    }

    fn is_deliberate_agreement(&self, gaze: &GazeData, reaction: &ReactionData) -> bool {
        gaze.gaze_entropy > self.gaze_entropy_threshold
            && reaction.reaction_time > self.deliberation_time_threshold
            && gaze.critical_zone_time > self.critical_attention_threshold
    }

    fn is_rubber_stamp(&self, gaze: &GazeData, reaction: &ReactionData) -> bool {
        gaze.gaze_entropy < self.low_gaze_threshold
            && reaction.reaction_time < self.snap_decision_threshold
    }
}
```

### 5.3 光速を利用した分散検証

```rust
impl DistributedVerifier {
    /// 因果律に基づく改ざん検出
    pub fn verify_causality(
        &self,
        events: &[DistributedEvent],
        time_verifier: &dyn TimeVerification,
    ) -> Result<bool, VerifyError> {
        for (i, e1) in events.iter().enumerate() {
            for e2 in events.iter().skip(i + 1) {
                // 地点間の最小物理遅延
                let min_delay = time_verifier.min_delay(e1.location, e2.location);

                // 実測された時間差
                let observed_diff = (e2.timestamp - e1.timestamp).abs();

                // 物理的に不可能な時間差を検出
                if observed_diff < min_delay {
                    return Err(VerifyError::CausalityViolation {
                        event1: e1.event_id,
                        event2: e2.event_id,
                        observed: observed_diff,
                        minimum: min_delay,
                    });
                }
            }
        }

        Ok(true)
    }

    /// 複数ノードでのタイムスタンプ整合性検証
    pub fn verify_consistency(
        &self,
        events: &[DistributedEvent],
    ) -> Result<bool, VerifyError> {
        // 各ノードの署名を検証
        for event in events {
            self.verify_node_signature(event)?;
        }

        // イベント間のハッシュチェーンを検証
        self.verify_hash_chain(events)?;

        Ok(true)
    }
}
```

### 5.4 サイドチャネル検証

```rust
impl SidechannelMonitor {
    /// サイドチャネルとログの整合性を検証
    pub fn verify_log_authenticity(
        &self,
        claimed_operations: &[Operation],
        power_data: &PowerProfile,
        em_data: &EmSpectrum,
    ) -> Result<bool, VerifyError> {
        // 操作内容から期待される電力プロファイルを生成
        let expected_power = self.generate_expected_profile(claimed_operations);

        // 実測との相関を計算
        let power_correlation = self.correlate(power_data, &expected_power);

        if power_correlation < self.correlation_threshold {
            return Err(VerifyError::SidechannelMismatch {
                correlation: power_correlation,
                threshold: self.correlation_threshold,
            });
        }

        // 電磁波スペクトルも同様に検証
        let expected_em = self.generate_expected_em(claimed_operations);
        let em_correlation = self.correlate_em(em_data, &expected_em);

        if em_correlation < self.em_threshold {
            return Err(VerifyError::EmMismatch);
        }

        Ok(true)
    }
}
```

### 5.5 警告と介入

```rust
impl AlertManager {
    /// 警告レベルに応じた介入
    pub fn handle_alert(
        &self,
        assessment: &DominationAssessment,
    ) -> Result<InterventionAction, AlertError> {
        match assessment.alert_level {
            AlertLevel::Normal => Ok(InterventionAction::None),

            AlertLevel::Yellow => {
                // 監視強化
                self.increase_monitoring_frequency();
                Ok(InterventionAction::IncreasedMonitoring)
            }

            AlertLevel::Orange => {
                // 認知的介入: AI回答に遅延を挿入
                self.insert_thinking_delay(self.forced_delay_duration);
                // 別管理者に警告
                self.notify_administrators(assessment)?;
                Ok(InterventionAction::CognitiveIntervention {
                    delay: self.forced_delay_duration,
                })
            }

            AlertLevel::Red => {
                // 物理的遮断（ハードウェアリレー）
                self.hardware_relay.disconnect_ai()?;
                self.notify_emergency(assessment)?;
                Ok(InterventionAction::PhysicalDisconnection)
            }
        }
    }
}
```

## 6. 物理層依存

| 機能 | 必要なトレイト/ハードウェア | 用途 |
|------|---------------------------|------|
| 一方向通信 | データダイオード | AIからの逆方向通信を物理的に遮断 |
| 時刻同期 | `TimeVerification` / GPS | 光速検証用の高精度タイムスタンプ |
| サイドチャネル | 電力モニター、スペアナ | ログの真正性検証 |
| 物理的遮断 | ハードウェアリレー | 緊急停止 |
| 視線追跡 | 独立カメラ | 視線エントロピー計測 |

## 7. 内部指標エンジン

| 指標 | 使用目的 |
|-----------------|---------|
| D* (乖離度) | 人間判断とAI提案の乖離度 |
| SRCI (修復サイクル) | 問題指摘→修正サイクルの追跡 |

## 8. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_agreement_high_domination() {
        let calc = DecisionEntropyCalculator::new(2.0);  // H_max = 2.0

        // 完全にAI追従
        let decisions = vec![Decision::A, Decision::A, Decision::B, Decision::B];
        let proposals = vec![Proposal::A, Proposal::A, Proposal::B, Proposal::B];

        let entropy = calc.calculate(&decisions, &proposals);

        // H(X|Y) ≈ 0 → 支配度 ≈ 1.0
        assert!(entropy.domination_score > 0.9);
    }

    #[test]
    fn test_deliberate_agreement_reduces_score() {
        let adjuster = DominationAdjuster::new();

        let high_entropy = DecisionEntropy {
            domination_score: 0.9,
            ..Default::default()
        };

        let deliberate_gaze = GazeData {
            gaze_entropy: 2.5,  // 高い（複数ゾーンを見ている）
            critical_zone_time: Duration::from_secs(10),
            ..Default::default()
        };

        let slow_reaction = ReactionData {
            reaction_time: Duration::from_secs(30),  // 30秒熟考
            ..Default::default()
        };

        let adjusted = adjuster.adjust(&high_entropy, &deliberate_gaze, &slow_reaction);

        // 熟考の結果なので、支配度は割り引かれる
        assert!(adjusted < high_entropy.domination_score);
    }

    #[test]
    fn test_causality_violation_detected() {
        let verifier = DistributedVerifier::new();
        let time = MockTimeVerification::new();

        // 東京とニューヨーク間の最小遅延: 50ms
        time.set_min_delay(Location::Tokyo, Location::NewYork, Duration::from_millis(50));

        // 物理的に不可能な同時イベント
        let events = vec![
            DistributedEvent {
                location: Location::Tokyo,
                timestamp: Timestamp::from_millis(1000),
                ..Default::default()
            },
            DistributedEvent {
                location: Location::NewYork,
                timestamp: Timestamp::from_millis(1010),  // 10ms差は不可能
                ..Default::default()
            },
        ];

        let result = verifier.verify_causality(&events, &time);
        assert!(matches!(result, Err(VerifyError::CausalityViolation { .. })));
    }
}
```
