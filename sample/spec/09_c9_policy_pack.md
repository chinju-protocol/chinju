# C9: 国地域ポリシーパックによる制約差し替えシステム - 詳細設計

## 1. 目的

国または地域ごとに異なる規制・監査・データ取扱い要件を、共通プロトコルの不変要素を損なわずに差し替え可能にする。

## 2. 核心アイデア

- 法域の差分を「ポリシーパック」として機械可読に表現
- 拡張要素の権限をポリシーに基づいてゲート
- 適用されたポリシーを認証証明書で追跡可能に
- 公開レジストリで第三者検証を可能に

## 3. ソフトウェア構成

```
c9_policy_pack/
├── pack/
│   ├── mod.rs
│   ├── definition.rs      # ポリシーパック定義
│   ├── storage.rs         # 保存
│   └── loader.rs          # 読み込み
├── gate/
│   ├── mod.rs
│   ├── judge.rs           # ゲート判定
│   ├── permission.rs      # 権限管理
│   └── condition.rs       # 条件付き許可
├── extension/
│   ├── mod.rs
│   ├── manifest.rs        # 拡張マニフェスト
│   └── installer.rs       # インストーラ
├── certification/
│   ├── mod.rs
│   ├── certificate.rs     # 証明書
│   ├── issuer.rs          # 発行
│   └── verifier.rs        # 検証
└── registry/
    ├── mod.rs
    ├── public.rs          # 公開レジストリ
    └── audit.rs           # 監査ログ
```

## 4. 主要なデータ型

```rust
/// ポリシーパック
pub struct PolicyPack {
    /// 識別子
    pub id: PolicyPackId,
    /// 版情報
    pub version: Version,
    /// 対象国・地域
    pub region: Region,
    /// 適用される法律・規制
    pub legal_basis: Vec<LegalReference>,
    /// デフォルト拒否方針
    pub default_deny: bool,
    /// 権限ルール
    pub permission_rules: Vec<PermissionRule>,
    /// 監査要件
    pub audit_requirements: AuditRequirements,
    /// 有効期限
    pub valid_until: Option<Timestamp>,
}

/// 権限ルール
pub struct PermissionRule {
    /// 対象権限
    pub permission: Permission,
    /// 判定
    pub decision: PermissionDecision,
    /// 条件（オプション）
    pub conditions: Vec<Condition>,
}

pub enum Permission {
    /// 外部送信
    ExternalTransmission,
    /// 外部保管
    ExternalStorage,
    /// 個人特定可能情報の取扱い
    PiiHandling,
    /// テキストログ利用
    TextLogUsage,
    /// イベント時刻列利用
    EventTimelineUsage,
    /// 生体情報取扱い
    BiometricHandling,
    /// カスタム権限
    Custom(String),
}

pub enum PermissionDecision {
    /// 許可
    Allow,
    /// 拒否
    Deny,
    /// 条件付き許可
    AllowIf { conditions: Vec<Condition> },
}

pub enum Condition {
    /// ユーザー同意が必要
    RequireConsent { consent_type: ConsentType },
    /// 匿名化が必要
    RequireAnonymization,
    /// 特定のサーバーのみ
    RestrictToServers { whitelist: Vec<ServerId> },
    /// 保管期間制限
    RetentionLimit { max_days: u32 },
    /// 監査ログ必須
    RequireAuditLog,
    /// 暗号化必須
    RequireEncryption,
}

/// 拡張マニフェスト
pub struct ExtensionManifest {
    /// 拡張ID
    pub id: ExtensionId,
    /// 名前
    pub name: String,
    /// 要求する権限
    pub required_permissions: Vec<Permission>,
    /// オプション権限
    pub optional_permissions: Vec<Permission>,
    /// 署名
    pub signature: Signature,
}

/// ゲート判定結果
pub enum GateResult {
    /// 許可
    Allow,
    /// 拒否
    Deny { reason: String },
    /// 条件付き許可
    ConditionalAllow { conditions: Vec<Condition> },
}

/// 認証証明書
pub struct Certificate {
    /// 証明書ID
    pub id: CertificateId,
    /// 対象（実装識別子）
    pub subject: ImplementationId,
    /// 認証ポリシー
    pub certification_policy: CertificationPolicyId,
    /// 適用ポリシーパック
    pub applied_policy_pack: PolicyPackId,
    /// ポリシーパック版
    pub policy_version: Version,
    /// プロトコル版
    pub protocol_version: Version,
    /// 状態
    pub status: CertificateStatus,
    /// 発行日
    pub issued_at: Timestamp,
    /// 有効期限
    pub expires_at: Timestamp,
    /// 署名
    pub signature: Signature,
}

pub enum CertificateStatus {
    Active,
    Suspended { reason: String },
    Revoked { reason: String },
    Expired,
}
```

## 5. アルゴリズム

### 5.1 ゲート判定

```rust
impl GateJudge {
    /// 拡張要素の導入可否を判定
    pub fn evaluate(
        &self,
        extension: &ExtensionManifest,
        policy: &PolicyPack,
    ) -> GateResult {
        let mut all_conditions = Vec::new();

        // 要求された各権限をチェック
        for permission in &extension.required_permissions {
            match self.check_permission(permission, policy) {
                GateResult::Allow => continue,
                GateResult::Deny { reason } => {
                    return GateResult::Deny {
                        reason: format!("Required permission {:?} denied: {}", permission, reason),
                    };
                }
                GateResult::ConditionalAllow { conditions } => {
                    all_conditions.extend(conditions);
                }
            }
        }

        if all_conditions.is_empty() {
            GateResult::Allow
        } else {
            GateResult::ConditionalAllow { conditions: all_conditions }
        }
    }

    /// 単一権限のチェック
    fn check_permission(
        &self,
        permission: &Permission,
        policy: &PolicyPack,
    ) -> GateResult {
        // ポリシーでルールを探す
        for rule in &policy.permission_rules {
            if rule.permission == *permission {
                return self.apply_rule(rule);
            }
        }

        // ルールがない場合はデフォルトポリシー
        if policy.default_deny {
            GateResult::Deny {
                reason: "No explicit rule, default is deny".into(),
            }
        } else {
            GateResult::Allow
        }
    }

    /// ルールを適用
    fn apply_rule(&self, rule: &PermissionRule) -> GateResult {
        match &rule.decision {
            PermissionDecision::Allow => GateResult::Allow,
            PermissionDecision::Deny => GateResult::Deny {
                reason: "Explicitly denied by policy".into(),
            },
            PermissionDecision::AllowIf { conditions } => {
                GateResult::ConditionalAllow {
                    conditions: conditions.clone(),
                }
            }
        }
    }
}
```

### 5.2 条件の適用

```rust
impl ConditionEnforcer {
    /// 条件を適用して実行コンテキストを生成
    pub fn apply_conditions(
        &self,
        conditions: &[Condition],
        context: &mut ExecutionContext,
    ) -> Result<(), ConditionError> {
        for condition in conditions {
            match condition {
                Condition::RequireConsent { consent_type } => {
                    if !context.has_consent(consent_type) {
                        return Err(ConditionError::ConsentRequired {
                            consent_type: consent_type.clone(),
                        });
                    }
                }
                Condition::RequireAnonymization => {
                    context.enable_anonymization();
                }
                Condition::RestrictToServers { whitelist } => {
                    context.set_server_whitelist(whitelist.clone());
                }
                Condition::RetentionLimit { max_days } => {
                    context.set_retention_limit(*max_days);
                }
                Condition::RequireAuditLog => {
                    context.enable_audit_logging();
                }
                Condition::RequireEncryption => {
                    context.require_encryption();
                }
            }
        }

        Ok(())
    }
}
```

### 5.3 証明書の発行

```rust
impl CertificateIssuer {
    /// 証明書を発行
    pub fn issue(
        &self,
        implementation: &ImplementationId,
        policy_pack: &PolicyPack,
        protocol_version: &Version,
    ) -> Result<Certificate, CertificationError> {
        // 適合性を検証
        self.verify_compliance(implementation, policy_pack)?;

        let certificate = Certificate {
            id: CertificateId::generate(),
            subject: implementation.clone(),
            certification_policy: self.policy_id.clone(),
            applied_policy_pack: policy_pack.id.clone(),
            policy_version: policy_pack.version.clone(),
            protocol_version: protocol_version.clone(),
            status: CertificateStatus::Active,
            issued_at: Timestamp::now(),
            expires_at: Timestamp::now() + self.validity_duration,
            signature: Signature::empty(),  // 後で署名
        };

        // 署名
        let signed = self.sign_certificate(certificate)?;

        // 公開レジストリに登録
        self.registry.register(&signed)?;

        Ok(signed)
    }

    /// 署名
    fn sign_certificate(&self, mut cert: Certificate) -> Result<Certificate, CertificationError> {
        let data = cert.serialize_for_signing();
        cert.signature = self.signing_key.sign(&data);
        Ok(cert)
    }
}
```

### 5.4 公開レジストリ

```rust
impl PublicRegistry {
    /// 証明書を登録
    pub fn register(&mut self, certificate: &Certificate) -> Result<(), RegistryError> {
        // 署名を検証
        self.verify_issuer_signature(certificate)?;

        // 登録
        self.certificates.insert(certificate.id.clone(), certificate.clone());

        // 監査ログに記録
        self.audit_log.append(AuditEvent::CertificateIssued {
            certificate_id: certificate.id.clone(),
            subject: certificate.subject.clone(),
            timestamp: Timestamp::now(),
        });

        Ok(())
    }

    /// 証明書を検証
    pub fn verify(&self, certificate_id: &CertificateId) -> Result<CertificateStatus, RegistryError> {
        let certificate = self.certificates.get(certificate_id)
            .ok_or(RegistryError::NotFound)?;

        // 署名を検証
        self.verify_signature(certificate)?;

        // 有効期限を確認
        if certificate.expires_at < Timestamp::now() {
            return Ok(CertificateStatus::Expired);
        }

        Ok(certificate.status.clone())
    }

    /// 証明書を失効
    pub fn revoke(
        &mut self,
        certificate_id: &CertificateId,
        reason: &str,
        authority: &Authority,
    ) -> Result<(), RegistryError> {
        // 権限を確認
        self.verify_revocation_authority(authority)?;

        let certificate = self.certificates.get_mut(certificate_id)
            .ok_or(RegistryError::NotFound)?;

        certificate.status = CertificateStatus::Revoked {
            reason: reason.to_string(),
        };

        // 監査ログに記録
        self.audit_log.append(AuditEvent::CertificateRevoked {
            certificate_id: certificate_id.clone(),
            reason: reason.to_string(),
            revoked_by: authority.id.clone(),
            timestamp: Timestamp::now(),
        });

        Ok(())
    }
}
```

### 5.5 監査ログ

```rust
impl AuditLog {
    /// イベントを追加
    pub fn append(&mut self, event: AuditEvent) {
        let entry = AuditEntry {
            sequence: self.next_sequence(),
            event,
            timestamp: Timestamp::now(),
            previous_hash: self.last_hash(),
        };

        let hash = entry.compute_hash();
        self.entries.push(entry);
        self.current_hash = hash;
    }

    /// ログの整合性を検証
    pub fn verify_integrity(&self) -> Result<(), IntegrityError> {
        let mut expected_hash = Hash::genesis();

        for entry in &self.entries {
            if entry.previous_hash != expected_hash {
                return Err(IntegrityError::HashMismatch {
                    sequence: entry.sequence,
                });
            }
            expected_hash = entry.compute_hash();
        }

        Ok(())
    }
}

pub enum AuditEvent {
    CertificateIssued { certificate_id: CertificateId, subject: ImplementationId, timestamp: Timestamp },
    CertificateRevoked { certificate_id: CertificateId, reason: String, revoked_by: AuthorityId, timestamp: Timestamp },
    PolicyPackUpdated { policy_pack_id: PolicyPackId, old_version: Version, new_version: Version, timestamp: Timestamp },
    GateDecision { extension_id: ExtensionId, policy_pack_id: PolicyPackId, result: GateResult, timestamp: Timestamp },
}
```

## 6. 物理層依存

**なし** - C9は純粋なソフトウェアコンポーネント。

## 7. 更新頻度

**中程度（データ追加）**

- 基盤コードは安定
- ポリシーパックデータは各国規制に応じて追加
- 新規制への対応は「データ追加」で対応

**推奨**: ポリシーパックデータは別リポジトリで管理

## 8. テスト戦略

### 8.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdpr_policy_requires_consent() {
        let policy = PolicyPack {
            id: PolicyPackId::new("policy.eu.gdpr"),
            region: Region::EU,
            default_deny: true,
            permission_rules: vec![
                PermissionRule {
                    permission: Permission::ExternalTransmission,
                    decision: PermissionDecision::AllowIf {
                        conditions: vec![
                            Condition::RequireConsent {
                                consent_type: ConsentType::Explicit,
                            },
                        ],
                    },
                    conditions: vec![],
                },
            ],
            ..Default::default()
        };

        let extension = ExtensionManifest {
            required_permissions: vec![Permission::ExternalTransmission],
            ..Default::default()
        };

        let judge = GateJudge::new();
        let result = judge.evaluate(&extension, &policy);

        assert!(matches!(result, GateResult::ConditionalAllow { conditions }
            if conditions.iter().any(|c| matches!(c, Condition::RequireConsent { .. }))));
    }

    #[test]
    fn test_china_policy_denies_external_transfer() {
        let policy = PolicyPack {
            id: PolicyPackId::new("policy.cn.csl"),
            region: Region::China,
            default_deny: true,
            permission_rules: vec![
                PermissionRule {
                    permission: Permission::ExternalTransmission,
                    decision: PermissionDecision::Deny,
                    conditions: vec![],
                },
            ],
            ..Default::default()
        };

        let extension = ExtensionManifest {
            required_permissions: vec![Permission::ExternalTransmission],
            ..Default::default()
        };

        let judge = GateJudge::new();
        let result = judge.evaluate(&extension, &policy);

        assert!(matches!(result, GateResult::Deny { .. }));
    }

    #[test]
    fn test_certificate_verification() {
        let mut registry = PublicRegistry::new();
        let issuer = CertificateIssuer::new_mock();

        let cert = issuer.issue(
            &ImplementationId::new("test-impl"),
            &PolicyPack::mock_gdpr(),
            &Version::new(1, 0, 0),
        ).unwrap();

        registry.register(&cert).unwrap();

        let status = registry.verify(&cert.id).unwrap();
        assert!(matches!(status, CertificateStatus::Active));
    }
}
```
