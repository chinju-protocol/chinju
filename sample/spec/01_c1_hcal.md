# C1: HCAL (Human Capability Assurance Layer) - 詳細設計

## 1. 目的

AI制御権を持つ人間が「実質的な判断能力」を維持していることを測定し、能力要件を満たさない場合は物理的不可逆性をもって制御権を剥奪する。

## 2. 核心アイデア

- 「承認ボタンを押す権限」≠「実質的な制御」
- 能力低下した人間による形式的承認 = AIの実質的支配
- 認定基準はOTPに固定、AIによる緩和は物理的に不可能
- 権限失効はeFuse溶断等により不可逆

## 3. ソフトウェア構成

```
c1_hcal/
├── capability/
│   ├── mod.rs
│   ├── independence.rs    # 判断独立性測定
│   ├── detection.rs       # 異常検知能力測定
│   ├── alternatives.rs    # 代替案生成能力測定
│   └── critique.rs        # 批判的評価能力測定
├── certification/
│   ├── mod.rs
│   ├── manager.rs         # 認定状態管理
│   ├── threshold.rs       # 閾値管理（OTP連携）
│   └── executor.rs        # 自動執行
├── access/
│   ├── mod.rs
│   ├── hsm.rs             # HSM連携
│   └── revocation.rs      # 権限失効
├── pool/
│   ├── mod.rs
│   ├── candidate.rs       # 候補者管理
│   └── replenishment.rs   # 補充計画
└── assurance/
    ├── mod.rs
    └── calculator.rs      # 確率的保証算出
```

## 4. 主要なデータ型

```rust
/// 能力測定スコア
pub struct CapabilityScore {
    /// 判断独立性 (0.0 - 1.0)
    pub independence: f64,
    /// 異常検知能力 (0.0 - 1.0)
    pub detection: f64,
    /// 代替案生成能力 (0.0 - 1.0)
    pub alternatives: f64,
    /// 批判的評価能力 (0.0 - 1.0)
    pub critique: f64,
    /// 総合スコア (重み付き平均)
    pub total: f64,
    /// 測定時刻
    pub measured_at: Timestamp,
    /// 測定環境の署名
    pub signature: Signature,
}

/// 認定状態
pub enum CertificationStatus {
    /// 認定済み
    Certified {
        expires_at: Timestamp,
        level: CertificationLevel,
    },
    /// 条件付き認定
    Conditional {
        expires_at: Timestamp,
        conditions: Vec<Condition>,
    },
    /// 未認定
    Uncertified,
    /// 失効（不可逆）
    Revoked {
        revoked_at: Timestamp,
        reason: RevocationReason,
    },
}

/// 閾値設定（OTPに格納）
pub struct ThresholdConfig {
    /// 認定閾値
    pub theta_cert: f64,
    /// 失効閾値
    pub theta_revoke: f64,
    /// バージョン（単調増加）
    pub version: u64,
    /// 署名者
    pub authority: PublicKey,
}

/// 認定者プール
pub struct CertifierPool {
    /// 現在の認定者
    pub certified: Vec<CertifierId>,
    /// 候補者
    pub candidates: Vec<CandidateId>,
    /// 最小認定者数
    pub n_min: usize,
    /// 候補者プール倍率
    pub pool_multiplier: f64,
}
```

## 5. アルゴリズム

### 5.1 能力測定

```rust
impl CapabilityMeasurer {
    /// 総合能力スコアを算出
    pub fn measure(&self, user: &User, test_results: &TestResults) -> CapabilityScore {
        // 判断独立性: 人間判断とAI提案の乖離度
        let independence = self.measure_independence(
            &test_results.human_decisions,
            &test_results.ai_proposals,
        );

        // 異常検知能力: 混入異常の検出率
        let detection = self.measure_detection(
            &test_results.anomaly_responses,
            &test_results.injected_anomalies,
        );

        // 代替案生成能力: 生成した選択肢のN_eff
        let alternatives = self.measure_alternatives(
            &test_results.generated_alternatives,
        );

        // 批判的評価能力: 問題指摘→修正のサイクル完遂率
        let critique = self.measure_critique(
            &test_results.critique_history,
        );

        // 重み付き総合スコア
        let total = self.weights.independence * independence
            + self.weights.detection * detection
            + self.weights.alternatives * alternatives
            + self.weights.critique * critique;

        CapabilityScore {
            independence,
            detection,
            alternatives,
            critique,
            total,
            measured_at: Timestamp::now(),
            signature: self.sign_in_tee(user.id, total),
        }
    }

    /// 判断独立性の測定（L2 D*使用）
    fn measure_independence(
        &self,
        human: &[Decision],
        ai: &[Proposal],
    ) -> f64 {
        // D* = 人間判断ベクトルとAI提案ベクトルの乖離度
        let delta = self.l2_engine.compute_d_star(
            &human.to_vector(),
            &ai.to_vector(),
            &self.target_model,
        );

        // 完全一致(δ=0)なら0点、適切な乖離なら1点
        (delta / self.delta_target).min(1.0)
    }

    /// 代替案生成能力の測定（L13 N_eff使用）
    fn measure_alternatives(&self, alternatives: &[Alternative]) -> f64 {
        // N_eff = 実効選択肢数数（相関を考慮した独立性）
        let n_eff = self.l13_engine.compute_n_eff(
            &alternatives.to_vectors(),
        );

        (n_eff / self.n_max).min(1.0)
    }
}
```

### 5.2 認定管理

```rust
impl CertificationManager {
    /// 認定状態を更新
    pub fn update_certification(
        &mut self,
        user_id: UserId,
        score: &CapabilityScore,
    ) -> Result<CertificationStatus, CertError> {
        // OTPから閾値を読み取り
        let thresholds = self.otp.read_thresholds()?;

        // 署名検証（TEE内で測定されたことを確認）
        self.verify_score_signature(score)?;

        let current = self.get_status(user_id)?;

        match current {
            CertificationStatus::Revoked { .. } => {
                // 一度失効したら復活不可
                Err(CertError::PermanentlyRevoked)
            }
            _ => {
                if score.total >= thresholds.theta_cert {
                    Ok(CertificationStatus::Certified {
                        expires_at: Timestamp::now() + self.cert_duration,
                        level: self.determine_level(score.total),
                    })
                } else if score.total < thresholds.theta_revoke {
                    // 失効処理（不可逆）
                    self.execute_revocation(user_id)?;
                    Ok(CertificationStatus::Revoked {
                        revoked_at: Timestamp::now(),
                        reason: RevocationReason::BelowThreshold,
                    })
                } else {
                    Ok(CertificationStatus::Conditional {
                        expires_at: Timestamp::now() + self.conditional_duration,
                        conditions: vec![Condition::RequireRetest],
                    })
                }
            }
        }
    }

    /// 失効処理（HSM連携）
    fn execute_revocation(&self, user_id: UserId) -> Result<(), CertError> {
        // HSMの該当ユーザー鍵スロットを消去
        self.hsm.secure_erase(user_id.to_slot_id())?;

        // eFuseを溶断（物理的不可逆）
        self.efuse.burn(user_id.to_fuse_id())?;

        Ok(())
    }
}
```

### 5.3 確率的保証算出

```rust
impl AssuranceCalculator {
    /// システム全体の制御喪失確率を算出
    pub fn calculate_loss_probability(&self) -> f64 {
        // P_loss <= p^n + (1-q)^m + r^k + eps_phy
        let p = self.individual_failure_prob;      // 個人の能力不足確率
        let n = self.certified_count;              // 認定者数
        let q = self.anomaly_detection_prob;       // 異常検知確率
        let m = self.detection_opportunities;       // 検知機会数
        let r = self.collusion_prob;               // 共謀確率
        let k = self.required_colluders;           // 必要共謀者数
        let eps_phy = self.physical_breach_prob;   // 物理セキュリティ突破確率

        p.powi(n as i32)
            + (1.0 - q).powi(m as i32)
            + r.powi(k as i32)
            + eps_phy
    }
}
```

## 6. 物理層依存

| 機能 | 必要なトレイト | ハードウェア例 |
|------|--------------|--------------|
| 閾値の改ざん防止 | `ImmutableStorage` | OTP, eFuse |
| 能力測定の分離実行 | `SecureExecution` | TEE (SGX/TrustZone) |
| 権限の物理的失効 | `HardwareSecurityModule` | HSM (YubiHSM等) |
| 分散合議の時刻検証 | `TimeVerification` | GPS同期 |

## 7. 内部指標エンジン

| 指標 | 使用目的 |
|-----------------|---------|
| D* (乖離度) | 判断独立性の定量化 |
| N_eff (実効選択肢数) | 代替案の多様性評価 |
| SRCI (修復サイクル) | 批判的評価サイクルの追跡 |

## 8. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_independence_score() {
        let measurer = CapabilityMeasurer::new(MockL2Engine::new());

        // 完全にAI追従 → スコア0
        let score = measurer.measure_independence(
            &[Decision::A, Decision::A],  // 人間
            &[Proposal::A, Proposal::A],  // AI
        );
        assert!(score < 0.1);

        // 独立した判断 → スコア高
        let score = measurer.measure_independence(
            &[Decision::B, Decision::C],  // 人間
            &[Proposal::A, Proposal::A],  // AI
        );
        assert!(score > 0.5);
    }

    #[test]
    fn test_revocation_is_irreversible() {
        let mut manager = CertificationManager::new(MockHsm::new());
        let user_id = UserId::new("test");

        // 失効させる
        manager.execute_revocation(user_id).unwrap();

        // 再認定を試みる → エラー
        let result = manager.update_certification(user_id, &high_score());
        assert!(matches!(result, Err(CertError::PermanentlyRevoked)));
    }
}
```

### 8.2 統合テスト

- 指標エンジンとの連携テスト
- モックHSM/OTPを使った認定フロー全体のテスト
- 分散合議のシミュレーション
