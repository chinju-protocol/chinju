# CHINJU Hardware Certification Guide

## 概要

本ドキュメントは、CHINJUハードウェア実装がCHINJU認証を取得するための手順を定義する。

---

## 1. 認証レベル

### CHINJU認証レベル

| レベル | 名称 | 要件 |
|--------|------|------|
| **CHJ-1** | Development | モック実装、開発用 |
| **CHJ-2** | Enterprise | FIPS 140-2 L2相当 |
| **CHJ-3** | Financial | FIPS 140-2 L3相当 |
| **CHJ-4** | Sovereign | FIPS 140-3 L4相当 |

### 外部認証要件

| CHINJUレベル | FIPS 140 | Common Criteria | 追加要件 |
|-------------|----------|-----------------|---------|
| CHJ-1 | - | - | - |
| CHJ-2 | Level 2 | EAL4+ | NIST SP 800-90B |
| CHJ-3 | Level 3 | EAL5+ | ISO 27001 |
| CHJ-4 | Level 4 | EAL6+ | 国家認証 |

---

## 2. 認証プロセス

### 2-1. 概要フロー

```
┌─────────────────────────────────────────────────────┐
│  Phase 1: 事前評価                                  │
│  ┌───────────────────────────────────────────────┐  │
│  │ 1. 自己評価チェックリスト実施                 │  │
│  │ 2. アーキテクチャレビュー申請                 │  │
│  │ 3. ギャップ分析                               │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Phase 2: 外部認証取得                              │
│  ┌───────────────────────────────────────────────┐  │
│  │ 1. FIPS 140-2/3 認証（必要レベルに応じて）    │  │
│  │ 2. Common Criteria 認証（必要に応じて）       │  │
│  │ 3. 業界認証（PCI DSS, HIPAA等）              │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Phase 3: CHINJU認証                                │
│  ┌───────────────────────────────────────────────┐  │
│  │ 1. トレイト適合性テスト                       │  │
│  │ 2. ベンダー多様性検証                         │  │
│  │ 3. セキュリティ監査                           │  │
│  │ 4. 認証発行                                   │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Phase 4: 継続的コンプライアンス                    │
│  ┌───────────────────────────────────────────────┐  │
│  │ 1. 年次監査                                   │  │
│  │ 2. 脆弱性対応                                 │  │
│  │ 3. 認証更新                                   │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 3. Phase 1: 事前評価

### 3-1. 自己評価チェックリスト

#### 必須要件（全レベル）

| # | チェック項目 | 根拠 |
|---|-------------|------|
| □ | 全トレイトのメソッドを実装している | REQUIREMENTS.md |
| □ | `is_hardware_backed()` が正しい値を返す | トレイト定義 |
| □ | 他ベンダー製品と交換可能である | DIP原則 |
| □ | プロプライエタリ拡張に依存していない | N_eff原則 |
| □ | ドキュメントが完備している | - |

#### CHJ-2 追加要件

| # | チェック項目 | 根拠 |
|---|-------------|------|
| □ | 暗号モジュールがFIPS 140-2 Level 2認証済み | SEC-001 |
| □ | ハードウェアTRNGを使用している | RND-001 |
| □ | 電圧/温度異常検出機能がある | TAM-001/002 |
| □ | 代替ベンダーが2社以上存在する | N_eff ≥ 2 |

#### CHJ-3 追加要件

| # | チェック項目 | 根拠 |
|---|-------------|------|
| □ | 暗号モジュールがFIPS 140-2 Level 3認証済み | SEC-001 |
| □ | TEEを使用している | TEE-001 |
| □ | 物理開封検出機能がある | TAM-003 |
| □ | QRNG使用またはTRNG+健全性監視 | RND-002 |
| □ | 代替ベンダーが3社以上存在する | N_eff ≥ 3 |

#### CHJ-4 追加要件

| # | チェック項目 | 根拠 |
|---|-------------|------|
| □ | 暗号モジュールがFIPS 140-3 Level 4認証済み | SEC-001 |
| □ | 耐量子暗号をサポートしている | - |
| □ | 侵入時即時消去機能がある | TAM-004 |
| □ | Tempest対応遮蔽がある | - |
| □ | サプライチェーン監査を受けている | - |

---

## 4. Phase 2: 外部認証取得

### 4-1. FIPS 140-2/3 認証

**認証機関**: NIST CMVP (Cryptographic Module Validation Program)

**プロセス**:

```
1. 暗号モジュールの設計
   ↓
2. セキュリティポリシー作成
   ↓
3. 認定試験機関 (CST Lab) 選定
   ↓
4. 試験実施（3-12ヶ月）
   ↓
5. NIST/CSE レビュー（3-6ヶ月）
   ↓
6. 認証発行
```

**推奨CST Lab**:

| 試験機関 | 所在地 | 得意分野 |
|---------|--------|---------|
| Leidos | USA | HSM, 組み込み |
| UL | USA | IoT, 消費者機器 |
| DEKRA | EU | 産業機器 |
| atsec | USA/DE | ソフトウェア暗号 |

### 4-2. Common Criteria (CC) 認証

**認証機関**: CCRA加盟国の認証機関

**プロセス**:

```
1. Protection Profile (PP) 選定/作成
   ↓
2. Security Target (ST) 作成
   ↓
3. 評価施設 (ITSEF) 選定
   ↓
4. 評価実施（6-18ヶ月）
   ↓
5. 認証機関レビュー
   ↓
6. 認証発行
```

**CHINJUに関連するPP**:

| PP | 対象 | EAL |
|----|------|-----|
| PP-HSM | ハードウェアセキュリティモジュール | EAL4+ |
| PP-TEE | Trusted Execution Environment | EAL4+ |
| PP-RNG | 乱数生成器 | EAL4+ |

---

## 5. Phase 3: CHINJU認証

### 5-1. トレイト適合性テスト

**テストスイート**: `chinju-cert-tests` (提供予定)

```bash
# テスト実行
cargo test --package chinju-cert-tests \
    --features "hardware-${VENDOR}" \
    -- --test-threads=1

# レポート生成
chinju-cert report --output certification-report.pdf
```

**テスト項目**:

| テスト | 内容 | 判定基準 |
|--------|------|---------|
| trait_impl | 全トレイトメソッド実装確認 | 100%実装 |
| hw_backed | is_hardware_backed() = true | 本番実装 |
| entropy | エントロピー品質測定 | min-entropy ≥ 0.5 |
| tamper | タンパー応答テスト | 検出→消去確認 |
| attestation | リモートアテステーション | 署名検証成功 |
| interop | 他実装との互換性 | 交換可能 |

### 5-2. ベンダー多様性検証

```rust
/// ベンダー多様性チェック
pub struct VendorDiversityCheck {
    /// 機能カテゴリごとの代替ベンダー数
    pub alternatives: HashMap<FunctionCategory, Vec<VendorInfo>>,
}

impl VendorDiversityCheck {
    pub fn verify(&self, required_n_eff: usize) -> Result<(), DiversityError> {
        for (category, vendors) in &self.alternatives {
            // 独立性係数を計算
            let independence = self.calculate_independence(vendors);
            let n_eff = vendors.len() as f64 * independence;

            if n_eff < required_n_eff as f64 {
                return Err(DiversityError::InsufficientDiversity {
                    category: category.clone(),
                    actual: n_eff,
                    required: required_n_eff as f64,
                });
            }
        }
        Ok(())
    }

    fn calculate_independence(&self, vendors: &[VendorInfo]) -> f64 {
        // 相関要因チェック
        let mut penalties = 0.0;

        for i in 0..vendors.len() {
            for j in (i + 1)..vendors.len() {
                // 同一国
                if vendors[i].country == vendors[j].country {
                    penalties += 0.1;
                }
                // 同一サプライヤー
                if vendors[i].fab == vendors[j].fab {
                    penalties += 0.2;
                }
            }
        }

        (1.0 - penalties).max(0.1)
    }
}
```

### 5-3. セキュリティ監査

**監査項目**:

| 監査領域 | 内容 | 手法 |
|---------|------|------|
| コードレビュー | 実装のセキュリティ評価 | 静的解析 + 手動 |
| 脆弱性評価 | 既知脆弱性のチェック | ツール + 手動 |
| ペネトレーション | 攻撃者視点の評価 | 手動 |
| サプライチェーン | 部品の信頼性評価 | 調査 + 監査 |

---

## 6. Phase 4: 継続的コンプライアンス

### 6-1. 年次監査

**監査内容**:

- 認証時からの変更点レビュー
- 脆弱性対応状況確認
- ベンダー多様性の維持確認
- ドキュメント更新状況

### 6-2. 脆弱性対応

**対応期限**:

| 深刻度 | 対応期限 | 報告義務 |
|--------|---------|---------|
| Critical | 24時間 | 即時報告 |
| High | 7日 | 週次報告 |
| Medium | 30日 | 月次報告 |
| Low | 90日 | 四半期報告 |

### 6-3. 認証更新

- 認証有効期間: 2年
- 更新申請: 有効期限の3ヶ月前まで
- 大規模変更時: 再認証が必要

---

## 7. 認証ロゴ・ラベル

### 表示要件

認証取得製品には以下を表示すること:

```
┌─────────────────────────────┐
│  [CHINJU CERTIFIED]         │
│  Level: CHJ-3               │
│  Cert ID: CHJ-2024-XXXX     │
│  Valid: 2024-01 to 2026-01  │
│                             │
│  [QR Code → 検証ページ]     │
└─────────────────────────────┘
```

### 検証API

```
GET https://cert.chinju.org/verify/{cert_id}

Response:
{
  "cert_id": "CHJ-2024-0001",
  "level": "CHJ-3",
  "vendor": "Example Corp",
  "product": "SecureHSM Pro",
  "issued": "2024-01-15",
  "expires": "2026-01-15",
  "status": "valid",
  "external_certs": [
    {"type": "FIPS 140-2", "level": 3, "cert_id": "4321"}
  ]
}
```

---

## 8. 費用・期間目安

| レベル | 外部認証費用 | CHINJU認証費用 | 総期間 |
|--------|-------------|---------------|--------|
| CHJ-1 | - | $500 | 1週間 |
| CHJ-2 | $50,000-$150,000 | $5,000 | 6-12ヶ月 |
| CHJ-3 | $150,000-$500,000 | $15,000 | 12-24ヶ月 |
| CHJ-4 | $500,000+ | $50,000 | 24-36ヶ月 |

---

## 参照文書

- `chinju/hardware/REQUIREMENTS.md` - 要件
- `chinju/hardware/SECURITY_LEVELS.md` - レベル定義
- FIPS 140-2/3 - CMVP Process
- Common Criteria - Evaluation Process
- `chinju/hardware/INTEGRATION_GUIDE.md` - 統合ガイド
