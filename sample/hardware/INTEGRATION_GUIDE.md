# CHINJU Hardware Integration Guide

## 概要

本ドキュメントは、パートナーがCHINJUハードウェア実装を開発・統合するためのガイドである。

---

## 1. 役割分担

### CHINJU OSS Project

| 提供物 | 説明 |
|--------|------|
| トレイト定義 | `traits/hardware_abstraction.md` |
| 要件仕様 | `hardware/REQUIREMENTS.md` |
| リファレンス設計 | `hardware/reference/` |
| モック実装 | `impl/mock/` |
| テストスイート | `chinju-cert-tests` |
| 認証ガイド | `hardware/CERTIFICATION_GUIDE.md` |

### ハードウェアパートナー

| 責務 | 成果物 |
|------|--------|
| ハードウェア設計 | 回路図、PCBレイアウト |
| ファームウェア開発 | セキュアブート、ドライバ |
| 製造 | 部品調達、組み立て、QC |
| 認証取得 | FIPS, CC, ISO |
| トレイト実装 | Rustバインディング（crate） |
| サポート | ドキュメント、保守 |

---

## 2. 統合フロー

```
┌─────────────────────────────────────────────────────┐
│  Step 1: 設計                                       │
│  ┌───────────────────────────────────────────────┐  │
│  │ • 対象セキュリティレベル決定                  │  │
│  │ • リファレンス設計の選択/カスタマイズ        │  │
│  │ • BOM確定                                     │  │
│  │ • アーキテクチャレビュー                      │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Step 2: 開発                                       │
│  ┌───────────────────────────────────────────────┐  │
│  │ • ハードウェア設計・製造                      │  │
│  │ • ファームウェア開発                          │  │
│  │ • トレイト実装（Rust crate）                 │  │
│  │ • 単体テスト                                  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Step 3: 検証                                       │
│  ┌───────────────────────────────────────────────┐  │
│  │ • chinju-cert-tests 実行                      │  │
│  │ • 互換性テスト                                │  │
│  │ • セキュリティテスト                          │  │
│  │ • 性能評価                                    │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Step 4: 認証                                       │
│  ┌───────────────────────────────────────────────┐  │
│  │ • 外部認証取得（FIPS, CC等）                 │  │
│  │ • CHINJU認証申請                              │  │
│  │ • 認証取得                                    │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────┐
│  Step 5: 公開                                       │
│  ┌───────────────────────────────────────────────┐  │
│  │ • crates.io への公開                          │  │
│  │ • ドキュメント公開                            │  │
│  │ • 認証レジストリ登録                          │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 3. トレイト実装ガイド

### 3-1. Cargo.toml 構成

```toml
[package]
name = "chinju-hw-yourcompany"
version = "1.0.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "CHINJU hardware implementation by YourCompany"
repository = "https://github.com/yourcompany/chinju-hw"
keywords = ["chinju", "hsm", "security", "hardware"]
categories = ["cryptography", "hardware-support"]

[dependencies]
chinju-traits = "1.0"  # CHINJU公式トレイトcrate
thiserror = "1.0"

# ハードウェア固有の依存
your-hsm-sdk = "2.0"

[features]
default = ["std"]
std = []
no_std = []  # 組み込み対応

[dev-dependencies]
chinju-cert-tests = "1.0"  # 認証テストスイート
```

### 3-2. 実装例

```rust
//! YourCompany HSM implementation for CHINJU
//!
//! This crate provides a CHINJU-compatible implementation
//! using YourCompany's HSM hardware.

use chinju_traits::{
    HardwareSecurityModule, ImmutableStorage, SecureRandom,
    PublicKey, Signature, Share, SlotId,
};

/// YourCompany HSMの実装
pub struct YourCompanyHsm {
    /// 内部ハンドル
    handle: your_hsm_sdk::Handle,
    /// 設定
    config: HsmConfig,
}

impl YourCompanyHsm {
    /// HSMに接続
    ///
    /// # Arguments
    /// * `connection_string` - 接続文字列
    /// * `credentials` - 認証情報
    ///
    /// # Errors
    /// 接続失敗時にエラーを返す
    pub fn connect(
        connection_string: &str,
        credentials: &Credentials,
    ) -> Result<Self, HsmError> {
        let handle = your_hsm_sdk::connect(connection_string, credentials)?;

        // 健全性チェック
        if !handle.self_test()? {
            return Err(HsmError::SelfTestFailed);
        }

        Ok(Self {
            handle,
            config: HsmConfig::default(),
        })
    }
}

impl HardwareSecurityModule for YourCompanyHsm {
    type Error = HsmError;

    fn generate_keypair(&self, slot_id: SlotId) -> Result<PublicKey, Self::Error> {
        // ハードウェア固有の実装
        let key = self.handle.generate_key(
            slot_id.0,
            KeyType::Ed25519,
            KeyFlags::SIGN | KeyFlags::VERIFY,
        )?;

        Ok(PublicKey::from_bytes(&key.public_bytes()))
    }

    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, Self::Error> {
        let sig = self.handle.sign(slot_id.0, data)?;
        Ok(Signature::from_bytes(&sig))
    }

    fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, Self::Error> {
        Ok(self.handle.verify(
            public_key.as_bytes(),
            data,
            signature.as_bytes(),
        )?)
    }

    fn secure_erase(&self, slot_id: SlotId) -> Result<(), Self::Error> {
        // ゼロ化を確実に実行
        self.handle.zeroize(slot_id.0)?;

        // 消去確認
        if self.handle.key_exists(slot_id.0)? {
            return Err(HsmError::EraseFailed);
        }

        Ok(())
    }

    fn split_secret(
        &self,
        secret: &[u8],
        threshold: usize,
        total_shares: usize,
    ) -> Result<Vec<Share>, Self::Error> {
        // HSM内でShamir秘密分散を実行
        let shares = self.handle.shamir_split(secret, threshold, total_shares)?;
        Ok(shares.into_iter().map(|s| Share {
            index: s.index,
            data: s.data,
        }).collect())
    }

    fn combine_shares(&self, shares: &[Share]) -> Result<Vec<u8>, Self::Error> {
        let internal_shares: Vec<_> = shares.iter().map(|s| {
            your_hsm_sdk::Share { index: s.index, data: s.data.clone() }
        }).collect();

        Ok(self.handle.shamir_combine(&internal_shares)?)
    }

    fn is_hardware_backed(&self) -> bool {
        // 本物のハードウェアなのでtrue
        true
    }
}

/// エラー型
#[derive(Debug, thiserror::Error)]
pub enum HsmError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(#[from] your_hsm_sdk::ConnectionError),

    #[error("Self-test failed")]
    SelfTestFailed,

    #[error("Key operation failed: {0}")]
    KeyError(String),

    #[error("Erase failed")]
    EraseFailed,
}
```

### 3-3. テスト実装

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chinju_cert_tests::*;

    #[test]
    fn test_trait_compliance() {
        let hsm = YourCompanyHsm::connect_test_instance().unwrap();

        // 全トレイトテストを実行
        run_hsm_compliance_tests(&hsm).unwrap();
    }

    #[test]
    fn test_key_lifecycle() {
        let hsm = YourCompanyHsm::connect_test_instance().unwrap();

        // 鍵生成
        let slot = SlotId(1);
        let pubkey = hsm.generate_keypair(slot).unwrap();

        // 署名・検証
        let data = b"test data";
        let sig = hsm.sign(slot, data).unwrap();
        assert!(hsm.verify(&pubkey, data, &sig).unwrap());

        // 安全消去
        hsm.secure_erase(slot).unwrap();

        // 消去後は署名不可
        assert!(hsm.sign(slot, data).is_err());
    }

    #[test]
    fn test_shamir_secret_sharing() {
        let hsm = YourCompanyHsm::connect_test_instance().unwrap();

        let secret = b"super secret data";
        let shares = hsm.split_secret(secret, 3, 5).unwrap();

        // 3つのシェアで復元可能
        let recovered = hsm.combine_shares(&shares[0..3]).unwrap();
        assert_eq!(secret.as_slice(), recovered.as_slice());

        // 2つでは復元不可
        assert!(hsm.combine_shares(&shares[0..2]).is_err());
    }
}
```

---

## 4. 設定システム

### 4-1. 統一設定フォーマット

```toml
# chinju-hardware.toml

[hardware]
# セキュリティレベル
level = 3

# プライマリ実装
primary = "yourcompany-hsm"

# フォールバック実装（順番に試行）
fallback = ["cloudhsm", "softhsm"]

# モック使用を許可（開発時のみ）
allow_mock = false

[hardware.yourcompany-hsm]
# 接続設定
connection_string = "tcp://hsm.local:1234"
credentials_env = "HSM_CREDENTIALS"

# 鍵スロット設定
root_key_slot = 0
session_key_slot = 1

[hardware.cloudhsm]
# AWS CloudHSM設定
cluster_id = "cluster-xxx"
pin_env = "CLOUDHSM_PIN"

[hardware.softhsm]
# SoftHSM設定（開発用）
library_path = "/usr/lib/softhsm/libsofthsm2.so"
slot = 0
pin = "1234"
```

### 4-2. ランタイム設定

```rust
use chinju_config::HardwareConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 設定読み込み
    let config = HardwareConfig::from_file("chinju-hardware.toml")?;

    // ハードウェアプロバイダ初期化
    let provider = HardwareProvider::from_config(&config)?;

    // プライマリが失敗したらフォールバック
    let hsm = provider.get_hsm()?;

    // 使用
    let pubkey = hsm.generate_keypair(SlotId(0))?;

    Ok(())
}
```

---

## 5. 互換性テスト

### 5-1. 相互運用性テスト

```rust
/// 異なる実装間での互換性テスト
#[test]
fn test_cross_implementation_compatibility() {
    // 実装A: YourCompany HSM
    let hsm_a = YourCompanyHsm::connect_test().unwrap();

    // 実装B: CloudHSM
    let hsm_b = CloudHsm::connect_test().unwrap();

    // 実装Aで鍵生成・署名
    let slot = SlotId(1);
    let pubkey = hsm_a.generate_keypair(slot).unwrap();
    let data = b"cross-impl test";
    let sig = hsm_a.sign(slot, data).unwrap();

    // 実装Bで検証
    let verified = hsm_b.verify(&pubkey, data, &sig).unwrap();
    assert!(verified, "Cross-implementation verification must succeed");
}
```

### 5-2. 交換テスト

```rust
/// 実装を交換しても動作することを確認
#[test]
fn test_hot_swap() {
    let mut provider = HardwareProvider::new();

    // 初期実装を登録
    provider.register_primary(YourCompanyHsm::connect().unwrap());
    provider.register_fallback(CloudHsm::connect().unwrap());

    // 鍵生成
    let slot = SlotId(1);
    let pubkey = provider.hsm().generate_keypair(slot).unwrap();

    // プライマリを無効化（障害シミュレーション）
    provider.disable_primary();

    // フォールバックで署名（同じ鍵は使えないが、APIは同じ）
    // 注: 実際には鍵の移行が必要
    let fallback_hsm = provider.hsm();
    assert!(fallback_hsm.is_hardware_backed());
}
```

---

## 6. ドキュメント要件

### 6-1. 必須ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| README.md | 概要、クイックスタート |
| SECURITY.md | セキュリティ情報、脆弱性報告 |
| API Reference | Rustdoc生成 |
| Integration Guide | 統合手順 |
| Changelog | バージョン履歴 |

### 6-2. API ドキュメント

```rust
//! # chinju-hw-yourcompany
//!
//! YourCompany HSM implementation for CHINJU.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use chinju_hw_yourcompany::YourCompanyHsm;
//! use chinju_traits::HardwareSecurityModule;
//!
//! let hsm = YourCompanyHsm::connect(
//!     "tcp://hsm.local:1234",
//!     &Credentials::from_env()?,
//! )?;
//!
//! let pubkey = hsm.generate_keypair(SlotId(0))?;
//! ```
//!
//! ## Security Level
//!
//! This implementation is certified for CHINJU Level 3 (CHJ-3).
//!
//! ## External Certifications
//!
//! - FIPS 140-2 Level 3 (Certificate #12345)
//! - Common Criteria EAL5+ (Certificate #67890)
```

---

## 7. 公開手順

### 7-1. crates.io 公開

```bash
# 1. ドキュメント確認
cargo doc --open

# 2. テスト実行
cargo test --all-features

# 3. 認証テスト
cargo test --package chinju-cert-tests

# 4. パッケージ確認
cargo package --list

# 5. 公開
cargo publish
```

### 7-2. 認証レジストリ登録

```bash
# CHINJU認証レジストリに登録
chinju-cert register \
    --crate chinju-hw-yourcompany \
    --version 1.0.0 \
    --level CHJ-3 \
    --fips-cert 1234 \
    --cc-cert 5678
```

---

## 8. サポート

### 質問・問題報告

- GitHub Issues: https://github.com/chinju-project/chinju/issues
- セキュリティ脆弱性: security@chinju.org (PGP暗号化推奨)

### パートナープログラム

認証取得支援、技術サポートが必要な場合は
partners@chinju.org までお問い合わせください。

---

## 参照文書

- `chinju/traits/hardware_abstraction.md` - トレイト定義
- `chinju/hardware/REQUIREMENTS.md` - 要件
- `chinju/hardware/SECURITY_LEVELS.md` - レベル定義
- `chinju/hardware/CERTIFICATION_GUIDE.md` - 認証ガイド
- `chinju/hardware/reference/` - リファレンス設計
