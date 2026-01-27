# C4: 権限分散型AI管理システム - 詳細設計

## 1. 目的

AIシステムのコアパラメータ（憲章、停止条件等）の変更権限を複数の独立した管理主体に分散させ、単独主体による恣意的な変更を構造的に不可能にする。

## 2. 核心アイデア

- 秘密分散法（Shamir's Secret Sharing）でマスター鍵を分割
- 閾値署名（k-of-n）で合意形成を強制
- 利害関係の異なる主体に鍵を配布（開発者、規制当局、ユーザー代表等）
- TPM/HSMで物理的に検証ロジックを保護

## 3. ソフトウェア構成

```
c4_distributed_auth/
├── key/
│   ├── mod.rs
│   ├── shamir.rs          # 秘密分散
│   ├── threshold.rs       # 閾値署名
│   └── distribution.rs    # 鍵配布
├── consensus/
│   ├── mod.rs
│   ├── proposal.rs        # 変更提案
│   ├── voting.rs          # 投票収集
│   └── quorum.rs          # 定足数判定
├── enforcement/
│   ├── mod.rs
│   ├── token.rs           # 変更許可トークン
│   └── killswitch.rs      # キルスイッチ
├── recovery/
│   ├── mod.rs
│   ├── deadman.rs         # デッドマンスイッチ
│   └── reissue.rs         # 鍵再発行
└── audit/
    ├── mod.rs
    └── log.rs             # 監査ログ
```

## 4. 主要なデータ型

```rust
/// 秘密分散のシェア
pub struct KeyShare {
    pub share_id: ShareId,
    pub share_data: Vec<u8>,
    pub holder: StakeholderId,
    pub threshold: usize,
    pub total_shares: usize,
}

/// ステークホルダー
pub struct Stakeholder {
    pub id: StakeholderId,
    pub name: String,
    pub category: StakeholderCategory,
    pub public_key: PublicKey,
    pub share: Option<KeyShare>,
}

pub enum StakeholderCategory {
    Developer,          // 開発企業
    RegulatorHome,      // 開発国の規制当局
    RegulatorForeign,   // 他国の規制当局
    Auditor,            // 第三者監査機関
    Academic,           // 研究者コミュニティ
    UserRepresentative, // ユーザー代表/DAO
}

/// 変更提案
pub struct ChangeProposal {
    pub proposal_id: ProposalId,
    pub change_type: ChangeType,
    pub content: ChangeContent,
    pub proposer: StakeholderId,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

pub enum ChangeType {
    ConstitutionAmendment,  // 憲章変更
    EmergencyShutdown,      // 緊急停止
    ParameterUpdate,        // パラメータ更新
    KeyRotation,            // 鍵ローテーション
}

/// 署名付き投票
pub struct SignedVote {
    pub voter: StakeholderId,
    pub proposal_id: ProposalId,
    pub vote: Vote,
    pub signature: Signature,
    pub timestamp: Timestamp,
}

pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

/// 変更許可トークン
pub struct ChangeToken {
    pub token_id: TokenId,
    pub proposal_id: ProposalId,
    pub authorized_change: ChangeContent,
    pub signatures: Vec<Signature>,
    pub valid_until: Timestamp,
}

/// 定足数ルール
pub struct QuorumRule {
    /// 変更タイプ
    pub change_type: ChangeType,
    /// 必要な署名数
    pub threshold: usize,
    /// 必須のステークホルダーカテゴリ
    pub required_categories: Vec<StakeholderCategory>,
    /// 拒否権を持つカテゴリ
    pub veto_categories: Vec<StakeholderCategory>,
}
```

## 5. アルゴリズム

### 5.1 秘密分散（Shamir's Secret Sharing）

```rust
impl ShamirSecretSharing {
    /// マスター鍵をn個のシェアに分割（k個で復元可能）
    pub fn split(
        &self,
        secret: &[u8],
        threshold: usize,
        total_shares: usize,
        hsm: &dyn HardwareSecurityModule,
    ) -> Result<Vec<KeyShare>, ShamirError> {
        // HSM内で秘密分散を実行
        let shares = hsm.split_secret(secret, threshold, total_shares)?;

        Ok(shares.into_iter().enumerate().map(|(i, share_data)| {
            KeyShare {
                share_id: ShareId::new(i),
                share_data,
                holder: StakeholderId::unassigned(),
                threshold,
                total_shares,
            }
        }).collect())
    }

    /// k個以上のシェアから秘密を復元
    pub fn combine(
        &self,
        shares: &[KeyShare],
        hsm: &dyn HardwareSecurityModule,
    ) -> Result<Vec<u8>, ShamirError> {
        if shares.len() < shares[0].threshold {
            return Err(ShamirError::InsufficientShares {
                provided: shares.len(),
                required: shares[0].threshold,
            });
        }

        let share_data: Vec<_> = shares.iter()
            .map(|s| s.share_data.clone())
            .collect();

        hsm.combine_shares(&share_data)
    }
}
```

### 5.2 合意形成プロトコル

```rust
impl ConsensusManager {
    /// 変更提案を作成
    pub fn create_proposal(
        &self,
        proposer: &Stakeholder,
        change_type: ChangeType,
        content: ChangeContent,
    ) -> Result<ChangeProposal, ConsensusError> {
        // 提案権限を確認
        self.verify_proposal_rights(proposer, &change_type)?;

        let proposal = ChangeProposal {
            proposal_id: ProposalId::generate(),
            change_type,
            content,
            proposer: proposer.id,
            created_at: Timestamp::now(),
            expires_at: Timestamp::now() + self.proposal_duration,
        };

        // 全ステークホルダーに通知
        self.notify_all_stakeholders(&proposal)?;

        Ok(proposal)
    }

    /// 投票を収集・検証
    pub fn collect_vote(
        &mut self,
        vote: SignedVote,
    ) -> Result<VoteStatus, ConsensusError> {
        // 署名を検証
        let stakeholder = self.get_stakeholder(vote.voter)?;
        self.verify_signature(&vote, &stakeholder.public_key)?;

        // 二重投票をチェック
        if self.has_voted(vote.voter, vote.proposal_id) {
            return Err(ConsensusError::DuplicateVote);
        }

        // 投票を記録
        self.votes.insert((vote.voter, vote.proposal_id), vote.clone());

        // 定足数をチェック
        self.check_quorum(vote.proposal_id)
    }

    /// 定足数判定
    fn check_quorum(&self, proposal_id: ProposalId) -> Result<VoteStatus, ConsensusError> {
        let proposal = self.get_proposal(proposal_id)?;
        let rule = self.get_quorum_rule(&proposal.change_type);

        let votes: Vec<_> = self.votes.iter()
            .filter(|((_, pid), _)| *pid == proposal_id)
            .map(|(_, v)| v)
            .collect();

        let approvals: Vec<_> = votes.iter()
            .filter(|v| matches!(v.vote, Vote::Approve))
            .collect();

        // 拒否権チェック
        for veto_category in &rule.veto_categories {
            let veto_votes: Vec<_> = votes.iter()
                .filter(|v| {
                    let holder = self.get_stakeholder(v.voter).unwrap();
                    holder.category == *veto_category && matches!(v.vote, Vote::Reject)
                })
                .collect();

            if !veto_votes.is_empty() {
                return Ok(VoteStatus::Vetoed { by: veto_category.clone() });
            }
        }

        // 必須カテゴリチェック
        for required in &rule.required_categories {
            let has_approval = approvals.iter().any(|v| {
                let holder = self.get_stakeholder(v.voter).unwrap();
                holder.category == *required
            });

            if !has_approval {
                return Ok(VoteStatus::Pending {
                    current: approvals.len(),
                    required: rule.threshold,
                    missing_categories: vec![required.clone()],
                });
            }
        }

        // 閾値チェック
        if approvals.len() >= rule.threshold {
            Ok(VoteStatus::Approved)
        } else {
            Ok(VoteStatus::Pending {
                current: approvals.len(),
                required: rule.threshold,
                missing_categories: vec![],
            })
        }
    }
}
```

### 5.3 変更許可トークンの発行

```rust
impl TokenIssuer {
    /// 承認された提案に対してトークンを発行
    pub fn issue_token(
        &self,
        proposal: &ChangeProposal,
        votes: &[SignedVote],
        hsm: &dyn HardwareSecurityModule,
    ) -> Result<ChangeToken, TokenError> {
        // シェアを収集
        let shares: Vec<_> = votes.iter()
            .filter(|v| matches!(v.vote, Vote::Approve))
            .map(|v| self.get_share(v.voter))
            .collect::<Result<_, _>>()?;

        // マスター鍵を復元（HSM内で）
        let master_key = hsm.combine_shares(&shares)?;

        // 変更内容に署名
        let token_data = TokenData {
            proposal_id: proposal.proposal_id,
            change: proposal.content.clone(),
            valid_until: Timestamp::now() + self.token_validity,
        };

        let signature = hsm.sign_with_key(&master_key, &token_data.serialize())?;

        // マスター鍵を即座に破棄（メモリから消去）
        hsm.secure_erase_temporary()?;

        Ok(ChangeToken {
            token_id: TokenId::generate(),
            proposal_id: proposal.proposal_id,
            authorized_change: proposal.content.clone(),
            signatures: votes.iter().map(|v| v.signature.clone()).collect(),
            valid_until: token_data.valid_until,
        })
    }
}
```

### 5.4 キルスイッチ

```rust
impl KillSwitch {
    /// 不正な変更を検出して停止
    pub fn enforce(
        &self,
        current_hash: Hash,
        expected_hash: Hash,
        tpm: &dyn TrustedPlatformModule,
    ) -> Result<(), KillSwitchError> {
        // TPMで憲章ハッシュを検証
        if !tpm.verify_hash(current_hash, expected_hash)? {
            // 不正な変更を検出
            self.trigger_shutdown(tpm)?;
            return Err(KillSwitchError::UnauthorizedChange);
        }

        Ok(())
    }

    /// 物理的停止を実行
    fn trigger_shutdown(&self, tpm: &dyn TrustedPlatformModule) -> Result<(), KillSwitchError> {
        // GPUへの電力供給を遮断
        tpm.cut_power_to_compute()?;

        // または復号鍵を破棄してモデルを無効化
        tpm.destroy_decryption_keys()?;

        Ok(())
    }
}
```

### 5.5 デッドマンスイッチ

```rust
impl DeadManSwitch {
    /// ハートビートを監視
    pub fn monitor(&mut self, stakeholder_id: StakeholderId) -> Option<RecoveryAction> {
        let last_heartbeat = self.heartbeats.get(&stakeholder_id)?;
        let silence_duration = Timestamp::now() - *last_heartbeat;

        if silence_duration > self.max_silence {
            // ステークホルダーが長期間無応答
            Some(RecoveryAction::InitiateKeyReissue {
                inactive: stakeholder_id,
                silence: silence_duration,
            })
        } else {
            None
        }
    }

    /// 鍵の再発行
    pub fn reissue_share(
        &mut self,
        inactive: StakeholderId,
        new_holder: StakeholderId,
        remaining_shares: &[KeyShare],
        hsm: &dyn HardwareSecurityModule,
    ) -> Result<KeyShare, RecoveryError> {
        // 通常より高い閾値を要求
        if remaining_shares.len() < self.recovery_threshold {
            return Err(RecoveryError::InsufficientShares);
        }

        // 秘密を復元
        let secret = hsm.combine_shares(remaining_shares)?;

        // 古いシェアを無効化
        self.revoke_share(inactive)?;

        // 新しいシェアを生成
        let new_shares = hsm.split_secret(&secret, self.threshold, self.total_shares)?;
        let new_share = new_shares.into_iter()
            .find(|s| s.share_id == inactive.to_share_id())
            .ok_or(RecoveryError::ShareGenerationFailed)?;

        // 新しい保持者に配布
        let mut assigned_share = new_share;
        assigned_share.holder = new_holder;

        Ok(assigned_share)
    }
}
```

## 6. 物理層依存

| 機能 | 必要なトレイト | ハードウェア例 |
|------|--------------|--------------|
| 秘密分散 | `HardwareSecurityModule` | HSM (YubiHSM, CloudHSM) |
| 署名検証 | `HardwareSecurityModule` | HSM |
| 憲章ハッシュ検証 | `TrustedPlatformModule` | TPM 2.0 |
| 物理的停止 | TPM電源制御 | TPM + 電源リレー |

## 7. テスト戦略

### 7.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_signature() {
        let hsm = MockHsm::new();
        let shamir = ShamirSecretSharing::new();

        let secret = b"master_key_data";
        let shares = shamir.split(secret, 3, 5, &hsm).unwrap();

        // 3つのシェアで復元可能
        let recovered = shamir.combine(&shares[0..3].to_vec(), &hsm).unwrap();
        assert_eq!(recovered, secret);

        // 2つのシェアでは復元不可
        let result = shamir.combine(&shares[0..2].to_vec(), &hsm);
        assert!(matches!(result, Err(ShamirError::InsufficientShares { .. })));
    }

    #[test]
    fn test_veto_power() {
        let mut manager = ConsensusManager::new();

        // 規制当局にveto権を設定
        manager.add_quorum_rule(QuorumRule {
            change_type: ChangeType::ConstitutionAmendment,
            threshold: 4,
            required_categories: vec![],
            veto_categories: vec![StakeholderCategory::RegulatorHome],
        });

        let proposal = manager.create_proposal(
            &developer,
            ChangeType::ConstitutionAmendment,
            content,
        ).unwrap();

        // 開発者が賛成
        manager.collect_vote(SignedVote {
            voter: developer.id,
            vote: Vote::Approve,
            ..Default::default()
        }).unwrap();

        // 規制当局が拒否 → veto発動
        let status = manager.collect_vote(SignedVote {
            voter: regulator.id,
            vote: Vote::Reject,
            ..Default::default()
        }).unwrap();

        assert!(matches!(status, VoteStatus::Vetoed { .. }));
    }
}
```
