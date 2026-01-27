# CHINJU Security Levels Definition

## 概要

CHINJUハードウェアは4段階のセキュリティレベルを定義する。
用途に応じて適切なレベルを選択し、過剰なコストを避けつつ必要なセキュリティを確保する。

---

## レベル定義

```
Level 4: 国家級        │ 軍事・国家機密・重要インフラ
─────────────────────────────────────────────────────
Level 3: 金融級        │ 銀行・決済・医療・エネルギー
─────────────────────────────────────────────────────
Level 2: 企業級        │ 一般企業・B2B・プロフェッショナル
─────────────────────────────────────────────────────
Level 1: 開発/教育     │ 開発・テスト・教育・デモ
─────────────────────────────────────────────────────
Level 0: モック        │ ソフトウェアのみ（物理保証なし）
```

---

## Level 0: Mock（開発専用）

**用途**: 開発・単体テスト・CI/CD

**特徴**:
- ハードウェアなし（純ソフトウェア実装）
- 暗号的保証なし
- 本番環境での使用禁止

**実装例**:
```rust
pub struct MockImmutableStorage {
    root_key: PublicKey,
    counter: AtomicU64,
}

impl ImmutableStorage for MockImmutableStorage {
    fn is_hardware_backed(&self) -> bool {
        false  // 常にfalse
    }
}
```

**警告**: モック使用時は起動時に警告を表示すること。

---

## Level 1: Development（開発/教育）

**用途**: プロトタイピング・教育・概念実証

**要件**:

| カテゴリ | 要件 | 実装例 |
|---------|------|--------|
| 暗号 | ソフトウェア暗号のみ | OpenSSL, ring |
| 乱数 | OS提供の乱数 | /dev/urandom |
| 鍵保管 | ファイルシステム（暗号化） | SQLite + SQLCipher |
| 認証 | なし | - |

**推奨ハードウェア**:

| コンポーネント | 製品例 | 価格帯 |
|--------------|--------|--------|
| 開発ボード | Raspberry Pi 4 | $50 |
| Secure Element | ATECC608B DevKit | $30 |
| USB HSM | SoftHSM (ソフトウェア) | 無料 |

**コスト**: ~$100

**制限**:
- 本番データの処理禁止
- 金銭取引の処理禁止
- 個人情報の処理禁止

---

## Level 2: Enterprise（企業級）

**用途**: 一般企業・B2Bサービス・非金融アプリケーション

**要件**:

| カテゴリ | 要件 | 認証 |
|---------|------|------|
| 暗号 | ハードウェア暗号 | FIPS 140-2 Level 2 |
| 乱数 | ハードウェアTRNG | NIST SP 800-90B |
| 鍵保管 | HSM/SE | CC EAL4+ |
| タンパー | 電圧/温度検出 | - |
| 認証 | FIPS 140-2 Level 2 | 必須 |

**推奨ハードウェア**:

| コンポーネント | 製品例 | ベンダー | 価格帯 |
|--------------|--------|---------|--------|
| USB HSM | YubiHSM 2 | Yubico | $650 |
| Secure Element | ATECC608B | Microchip | $1 |
| MCU | STM32L4 + TrustZone | ST | $5 |
| 代替HSM | SoftHSM + TPM | 各社 | $50 |

**ベンダー多様性（必須）**:

| 機能 | プライマリ | セカンダリ |
|------|-----------|-----------|
| HSM | YubiHSM 2 | AWS CloudHSM |
| SE | ATECC608B | NXP SE050 |
| MCU | STM32L4 | NXP LPC55S |

**コスト**: ~$700-$1,500

---

## Level 3: Financial（金融級）

**用途**: 銀行・決済・医療・エネルギー・重要インフラ

**要件**:

| カテゴリ | 要件 | 認証 |
|---------|------|------|
| 暗号 | ハードウェア暗号 | FIPS 140-2 Level 3 |
| 乱数 | QRNG推奨 | NIST SP 800-90B |
| 鍵保管 | Network HSM | FIPS 140-2 Level 3 |
| タンパー | 物理開封検出 | - |
| 実行環境 | TEE必須 | CC EAL5+ |
| 認証 | FIPS 140-2 Level 3 | 必須 |

**推奨ハードウェア**:

| コンポーネント | 製品例 | ベンダー | 価格帯 |
|--------------|--------|---------|--------|
| Network HSM | Luna Network HSM | Thales | $15,000+ |
| 代替HSM | AWS CloudHSM | AWS | $1.5/時間 |
| QRNG | Quantis PCIe | ID Quantique | $2,000 |
| TEE | Intel SGX対応サーバー | Intel | $5,000+ |
| タンパー筐体 | カスタム | 各社 | $1,000+ |

**ベンダー多様性（必須）**:

| 機能 | プライマリ | セカンダリ | ターシャリ |
|------|-----------|-----------|-----------|
| HSM | Thales Luna | AWS CloudHSM | Entrust nShield |
| TEE | Intel SGX | AMD SEV | ARM TrustZone |
| QRNG | ID Quantique | Quantis | TRNG fallback |

**コスト**: $20,000-$50,000

---

## Level 4: Sovereign（国家級）

**用途**: 軍事・情報機関・国家機密・戦略的インフラ

**要件**:

| カテゴリ | 要件 | 認証 |
|---------|------|------|
| 暗号 | 耐量子暗号対応 | FIPS 140-3 Level 4 |
| 乱数 | QRNG必須 | NIST SP 800-90B |
| 鍵保管 | オンプレミスHSM | FIPS 140-3 Level 4 |
| タンパー | 侵入即時消去 | - |
| 実行環境 | 専用TEE | CC EAL6+ |
| 物理 | 電磁シールド・Tempest | MIL-STD |
| 認証 | 国家認証 | 必須 |

**推奨ハードウェア**:

| コンポーネント | 要件 | 備考 |
|--------------|------|------|
| HSM | FIPS 140-3 Level 4 | オンプレミス必須 |
| QRNG | 量子光学方式 | 複数冗長構成 |
| TEE | カスタムシリコン | 専用設計 |
| 筐体 | Tempest対応 | 電磁遮蔽 |
| 施設 | SCIF準拠 | 物理セキュリティ |

**ベンダー多様性（必須）**:

| 機能 | 要件 |
|------|------|
| HSM | 国内製造または同盟国製造 |
| 半導体 | サプライチェーン監査済み |
| ファームウェア | ソースコード開示必須 |

**コスト**: $100,000+（施設除く）

---

## レベル選択ガイド

```
START
  │
  ├─ 本番環境か？
  │   └─ No → Level 0/1
  │
  ├─ 金銭・個人情報を扱うか？
  │   └─ No → Level 2
  │
  ├─ 規制対象か？（金融/医療/インフラ）
  │   └─ No → Level 2
  │   └─ Yes → Level 3
  │
  └─ 国家安全保障に関わるか？
      └─ Yes → Level 4
```

---

## レベル間の互換性

**上位互換**: Level N の実装は Level N-1 の要件を満たす。

**設定による切り替え**:

```toml
# config.toml

[security]
level = 3  # 1-4

[hardware.hsm]
# Level 2
level2_primary = "yubihsm"
level2_fallback = "cloudhsm"

# Level 3
level3_primary = "luna"
level3_fallback = "cloudhsm"

[hardware.tee]
level2_enabled = false
level3_enabled = true
provider = "sgx"  # or "sev", "trustzone"
```

---

## 認証マトリクス

| 認証 | Level 1 | Level 2 | Level 3 | Level 4 |
|------|---------|---------|---------|---------|
| FIPS 140-2/3 | - | Level 2 | Level 3 | Level 4 |
| Common Criteria | - | EAL4+ | EAL5+ | EAL6+ |
| NIST SP 800-90B | - | 推奨 | 必須 | 必須 |
| ISO 27001 | - | 推奨 | 必須 | 必須 |
| SOC 2 Type II | - | 推奨 | 必須 | 必須 |
| PCI DSS | - | - | 必須* | 必須* |
| HIPAA | - | - | 必須* | 必須* |

*該当業界のみ

---

## 参照文書

- `chinju/hardware/REQUIREMENTS.md` - 詳細要件
- `chinju/hardware/reference/` - レベル別リファレンス設計
- `chinju/hardware/CERTIFICATION_GUIDE.md` - 認証取得ガイド
