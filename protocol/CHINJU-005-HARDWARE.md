# CHINJU-005: Hardware Requirements（ハードウェア要件）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-01-31

---

## 1. 概要

本文書は、CHINJUプロトコルのハードウェア要件を定義する。

### 1.1 設計原則

```
原則 1: 物理法則への依存
  - ソフトウェアだけでは実現不可能な保証
  - 攻撃者の能力上限を物理法則で規定

原則 2: 段階的導入
  - すべてのハードウェアは必須ではない
  - セキュリティレベルに応じた選択

原則 3: 相互運用性
  - 特定ベンダーへの依存を避ける
  - 標準インターフェースを使用
```

---

## 2. セキュリティレベル

### 2.1 レベル定義

| レベル | 名称 | 用途 | 攻撃者想定 |
|--------|------|------|----------|
| L0 | Development | 開発・テスト | なし |
| L1 | Basic | 小規模運用 | スクリプトキディ |
| L2 | Standard | 一般運用 | 熟練ハッカー |
| L3 | Enterprise | 大規模運用 | 組織的攻撃 |
| L4 | Critical | 重要インフラ | 国家レベル |

### 2.2 レベル別ハードウェア要件

```
┌─────────────────────────────────────────────────────────────────┐
│                    Security Levels                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  L4 Critical    ┌───────────────────────────────────────────┐   │
│                 │ QRNG + HSM(FIPS 140-3 L3) + Data Diode   │   │
│                 │ + Physical Tamper Detection              │   │
│                 │ + Multi-Region Distribution              │   │
│                 └───────────────────────────────────────────┘   │
│                                   ▲                              │
│  L3 Enterprise  ┌────────────────┴──────────────────────────┐   │
│                 │ HSM(FIPS 140-2 L3) + TEE + TPM 2.0       │   │
│                 │ + Secure Boot                             │   │
│                 └───────────────────────────────────────────┘   │
│                                   ▲                              │
│  L2 Standard    ┌────────────────┴──────────────────────────┐   │
│                 │ TPM 2.0 + TEE (ARM TrustZone/Intel SGX)  │   │
│                 └───────────────────────────────────────────┘   │
│                                   ▲                              │
│  L1 Basic       ┌────────────────┴──────────────────────────┐   │
│                 │ Software TPM + Secure Enclave (iOS/Android)│   │
│                 └───────────────────────────────────────────┘   │
│                                   ▲                              │
│  L0 Development ┌────────────────┴──────────────────────────┐   │
│                 │ Software Emulation (Mock)                 │   │
│                 └───────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. ハードウェアカテゴリ

### 3.1 暗号モジュール（Cryptographic Modules）

#### 3.1.1 HSM (Hardware Security Module)

```yaml
目的: 暗号鍵の安全な保管と演算

要件:
  L2:
    - FIPS 140-2 Level 2 以上
    - PKCS#11 または JCE 対応
    - 例: YubiHSM 2, AWS CloudHSM

  L3:
    - FIPS 140-2 Level 3 以上
    - タンパー検知・ゼロ化
    - 例: Thales Luna, Entrust nShield

  L4:
    - FIPS 140-3 Level 3 以上
    - 物理破壊時の鍵消去
    - 例: Utimaco, Fortanix

インターフェース:
  - PKCS#11 2.40 以上
  - RSA 4096, ECDSA P-384, Ed25519 対応
  - AES-256-GCM 対応

性能要件:
  - 署名: ≥ 1000 ops/sec (ECDSA P-256)
  - 鍵生成: ≥ 100 keys/sec
  - レイテンシ: ≤ 10ms (署名)
```

#### 3.1.2 TPM (Trusted Platform Module)

```yaml
目的: プラットフォームの信頼性証明

要件:
  L1:
    - TPM 1.2 (ソフトウェア)

  L2:
    - TPM 2.0 (ハードウェア)
    - TCG 準拠

  L3:
    - TPM 2.0 (ディスクリート)
    - FIPS 140-2 Level 2

機能:
  - 測定済み起動 (Measured Boot)
  - 遠隔認証 (Remote Attestation)
  - 封印/開封 (Sealing/Unsealing)

インターフェース:
  - TCG TSS 2.0
  - tpm2-tools 互換
```

### 3.2 乱数生成（Random Number Generation）

#### 3.2.1 QRNG (Quantum Random Number Generator)

```yaml
目的: 予測不可能な真性乱数の生成

要件:
  L3:
    - NIST SP 800-90B 準拠
    - エントロピー: ≥ 1 bit/sample
    - 例: ID Quantique Quantis, QuintessenceLabs

  L4:
    - デバイス独立性証明
    - リアルタイム検証
    - 例: QuintessenceLabs qStream

性能要件:
  - 生成速度: ≥ 1 Mbps
  - レイテンシ: ≤ 1ms (256 bits)

検証:
  - NIST SP 800-22 テストスイート合格
  - リアルタイムエントロピー監視
```

#### 3.2.2 TRNG (True Random Number Generator)

```yaml
目的: QRNG の代替（L2 レベル）

要件:
  L2:
    - ハードウェア TRNG
    - エントロピー: ≥ 0.5 bit/sample
    - 例: Intel RDRAND, ARM TRNG

性能要件:
  - 生成速度: ≥ 100 Kbps
```

### 3.3 実行隔離（Execution Isolation）

#### 3.3.1 TEE (Trusted Execution Environment)

```yaml
目的: 機密コードの隔離実行

要件:
  L2:
    - ARM TrustZone または Intel SGX
    - メモリ暗号化

  L3:
    - Intel SGX2 または AMD SEV-SNP
    - リモートアテステーション
    - 例: Azure Confidential Computing

  L4:
    - ARM Confidential Compute Architecture (CCA)
    - マルチパーティ計算対応

機能:
  - 暗号化メモリ
  - シールドストレージ
  - リモート証明

制限事項:
  - サイドチャネル攻撃への注意（SGX）
  - 定期的なファームウェア更新
```

### 3.4 不可逆記憶（Immutable Storage）

#### 3.4.1 OTP (One-Time Programmable Memory)

```yaml
目的: 閾値・ハッシュの改ざん不可能な保存

要件:
  L3:
    - eFuse ベース
    - ≥ 256 bits
    - 例: Microchip ATECC608, NXP SE050

  L4:
    - 物理破壊検知付き
    - ≥ 1024 bits

特性:
  - 一度書いたら消去不可（物理法則）
  - 熱力学第二法則に基づく不可逆性

用途:
  - Genesis Block ハッシュ
  - 閾値設定
  - ルート公開鍵
```

#### 3.4.2 eFuse

```yaml
目的: 権限失効の物理的執行

要件:
  L3:
    - 個別溶断可能
    - ≥ 64 ヒューズ

  L4:
    - 溶断の第三者検証可能
    - 監査ログとの連携

特性:
  - 電気的に溶断（不可逆）
  - 読み取りのみ可能

用途:
  - ユーザー権限の失効
  - 緊急停止フラグ
```

### 3.5 物理セキュリティ（Physical Security）

#### 3.5.1 Data Diode

```yaml
目的: 情報流の一方向性保証

要件:
  L4:
    - ハードウェアデータダイオード
    - 逆方向通信の物理的不可能
    - 例: Waterfall Security, Owl Cyber Defense

特性:
  - 光学的または電気的な一方向性
  - 帯域: ≥ 1 Gbps

用途:
  - 監査ログの外部転送
  - AI 出力の監視系への送信
  - 安全な更新の受信
```

#### 3.5.2 Tamper Detection

```yaml
目的: 物理的侵入の検知

要件:
  L3:
    - 開封検知センサー
    - 環境異常検知（温度、電圧）

  L4:
    - アクティブシールド
    - 侵入時自動消去
    - 24時間監視

センサータイプ:
  - メッシュセンサー
  - 光センサー
  - 温度センサー
  - 電圧監視
```

### 3.6 時刻同期（Time Synchronization）

#### 3.6.1 GPS / GNSS

```yaml
目的: 改ざん困難な時刻取得

要件:
  L3:
    - GPS 受信機
    - アンテナ設置

  L4:
    - マルチ GNSS (GPS + Galileo + GLONASS)
    - スプーフィング検知

用途:
  - 分散合意のタイムスタンプ
  - 証明書の有効期限検証
  - 監査ログの時刻証明
```

---

## 4. トレイト定義（ソフトウェアインターフェース）

### 4.1 共通トレイト

```rust
// chinju/traits/hardware.rs

/// 信頼の根拠を示すトレイト
pub trait TrustRoot: Send + Sync {
    /// ハードウェアに支えられているか
    fn is_hardware_backed(&self) -> bool;

    /// セキュリティレベル
    fn security_level(&self) -> SecurityLevel;

    /// ハードウェア証明を取得
    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    L0Development = 0,
    L1Basic = 1,
    L2Standard = 2,
    L3Enterprise = 3,
    L4Critical = 4,
}
```

### 4.2 HSM トレイト

```rust
/// HSM インターフェース
pub trait HardwareSecurityModule: TrustRoot {
    type Error: std::error::Error;

    /// 鍵ペアを生成
    fn generate_key_pair(
        &self,
        algorithm: KeyAlgorithm,
        label: &str,
    ) -> Result<KeyHandle, Self::Error>;

    /// 署名を生成
    fn sign(
        &self,
        key_handle: &KeyHandle,
        data: &[u8],
    ) -> Result<Signature, Self::Error>;

    /// 署名を検証
    fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, Self::Error>;

    /// 鍵を安全に消去
    fn secure_erase(&self, key_handle: &KeyHandle) -> Result<(), Self::Error>;

    /// 秘密分散の鍵シェアを生成
    fn generate_shares(
        &self,
        key_handle: &KeyHandle,
        threshold: u32,
        total: u32,
    ) -> Result<Vec<KeyShare>, Self::Error>;

    /// 鍵シェアから鍵を復元
    fn reconstruct_key(
        &self,
        shares: &[KeyShare],
    ) -> Result<KeyHandle, Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub enum KeyAlgorithm {
    Ed25519,
    EcdsaP256,
    EcdsaP384,
    Rsa4096,
}
```

### 4.3 OTP/eFuse トレイト

```rust
/// 不可逆ストレージ インターフェース
pub trait ImmutableStorage: TrustRoot {
    type Error: std::error::Error;

    /// データを読み取り
    fn read(&self, slot: SlotId) -> Result<Vec<u8>, Self::Error>;

    /// データを書き込み（一度だけ、不可逆）
    fn burn(&self, slot: SlotId, data: &[u8]) -> Result<(), Self::Error>;

    /// スロットが書き込み済みか確認
    fn is_burned(&self, slot: SlotId) -> Result<bool, Self::Error>;

    /// 利用可能なスロット数
    fn available_slots(&self) -> Result<usize, Self::Error>;

    /// スロットのサイズ（バイト）
    fn slot_size(&self) -> usize;
}

/// eFuse 専用インターフェース
pub trait FuseControl: ImmutableStorage {
    /// 個別ヒューズを溶断
    fn burn_fuse(&self, fuse_id: FuseId) -> Result<(), Self::Error>;

    /// ヒューズの状態を確認
    fn is_fuse_blown(&self, fuse_id: FuseId) -> Result<bool, Self::Error>;
}
```

### 4.4 乱数生成トレイト

```rust
/// 乱数生成インターフェース
pub trait RandomSource: TrustRoot {
    type Error: std::error::Error;

    /// 乱数を生成
    fn generate(&self, length: usize) -> Result<Vec<u8>, Self::Error>;

    /// エントロピー品質を取得
    fn entropy_quality(&self) -> Result<EntropyQuality, Self::Error>;

    /// 乱数生成器の健全性チェック
    fn health_check(&self) -> Result<HealthStatus, Self::Error>;
}

#[derive(Debug, Clone)]
pub struct EntropyQuality {
    /// 1サンプルあたりのエントロピー（ビット）
    pub bits_per_sample: f64,
    /// 最終検証時刻
    pub last_verified: Timestamp,
    /// NIST テスト合格
    pub nist_compliant: bool,
}

/// QRNG 専用インターフェース
pub trait QuantumRandomSource: RandomSource {
    /// 量子力学的な乱数であることを証明
    fn get_quantum_proof(&self) -> Result<QuantumProof, Self::Error>;
}
```

### 4.5 TEE トレイト

```rust
/// 信頼実行環境インターフェース
pub trait SecureExecution: TrustRoot {
    type Error: std::error::Error;

    /// エンクレーブを作成
    fn create_enclave(
        &self,
        code: &[u8],
        config: EnclaveConfig,
    ) -> Result<EnclaveHandle, Self::Error>;

    /// エンクレーブ内で関数を実行
    fn execute_in_enclave<T, R>(
        &self,
        enclave: &EnclaveHandle,
        function: T,
        input: &[u8],
    ) -> Result<R, Self::Error>
    where
        T: Fn(&[u8]) -> R;

    /// リモートアテステーション
    fn remote_attestation(
        &self,
        enclave: &EnclaveHandle,
        challenge: &[u8],
    ) -> Result<AttestationReport, Self::Error>;

    /// シールドストレージへの書き込み
    fn seal_data(
        &self,
        enclave: &EnclaveHandle,
        data: &[u8],
    ) -> Result<SealedData, Self::Error>;

    /// シールドストレージからの読み取り
    fn unseal_data(
        &self,
        enclave: &EnclaveHandle,
        sealed: &SealedData,
    ) -> Result<Vec<u8>, Self::Error>;

    /// エンクレーブを破棄
    fn destroy_enclave(&self, enclave: EnclaveHandle) -> Result<(), Self::Error>;
}
```

### 4.6 データダイオード トレイト

```rust
/// 一方向通信インターフェース
pub trait UnidirectionalChannel: TrustRoot {
    type Error: std::error::Error;

    /// データを送信（一方向）
    fn send(&self, data: &[u8]) -> Result<(), Self::Error>;

    /// 送信可能か確認
    fn is_ready(&self) -> bool;

    /// 帯域幅を取得
    fn bandwidth(&self) -> Bandwidth;

    /// 一方向性を検証（受信できないことを確認）
    fn verify_unidirectional(&self) -> Result<bool, Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct Bandwidth {
    pub bits_per_second: u64,
}
```

---

## 5. モック実装

### 5.1 開発用モック

```rust
// chinju/hardware/mock.rs

/// 開発用 HSM モック
pub struct MockHsm {
    keys: HashMap<KeyHandle, KeyPair>,
}

impl TrustRoot for MockHsm {
    fn is_hardware_backed(&self) -> bool {
        false  // ソフトウェア実装
    }

    fn security_level(&self) -> SecurityLevel {
        SecurityLevel::L0Development
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        Ok(HardwareAttestation {
            trust_level: TrustLevel::Mock,
            hardware_type: "MockHSM".to_string(),
            ..Default::default()
        })
    }
}

impl HardwareSecurityModule for MockHsm {
    // ... ソフトウェアによる暗号演算の実装
}

/// 開発用 OTP モック（ファイルベース）
pub struct MockOtp {
    storage_path: PathBuf,
}

impl ImmutableStorage for MockOtp {
    fn burn(&self, slot: SlotId, data: &[u8]) -> Result<(), Self::Error> {
        // 警告: 実際には削除可能（開発用）
        let path = self.storage_path.join(slot.to_string());
        if path.exists() {
            return Err(OtpError::AlreadyBurned);
        }
        fs::write(path, data)?;
        Ok(())
    }
}
```

---

## 6. 認定ハードウェアリスト

### 6.1 HSM

| 製品 | ベンダー | 認証 | レベル |
|------|---------|------|--------|
| YubiHSM 2 | Yubico | FIPS 140-2 L3 | L2-L3 |
| Luna Network HSM | Thales | FIPS 140-2 L3 | L3-L4 |
| nShield Connect | Entrust | FIPS 140-2 L3 | L3-L4 |
| CloudHSM | AWS | FIPS 140-2 L3 | L3 |

### 6.2 TPM

| 製品 | ベンダー | 規格 | レベル |
|------|---------|------|--------|
| TPM 2.0 (Intel PTT) | Intel | TCG 2.0 | L2 |
| TPM 2.0 (AMD fTPM) | AMD | TCG 2.0 | L2 |
| OPTIGA TPM | Infineon | TCG 2.0 | L2-L3 |

### 6.3 QRNG

| 製品 | ベンダー | 速度 | レベル |
|------|---------|------|--------|
| Quantis | ID Quantique | 4-16 Mbps | L3 |
| qStream | QuintessenceLabs | 1 Gbps | L4 |
| QRNG USB | PQRNG | 1 Mbps | L3 |

### 6.4 TEE

| 製品 | ベンダー | 技術 | レベル |
|------|---------|------|--------|
| Intel SGX | Intel | SGX2 | L2-L3 |
| AMD SEV-SNP | AMD | SEV | L3 |
| ARM TrustZone | ARM | TrustZone | L2 |
| AWS Nitro Enclaves | AWS | Nitro | L3 |

---

## 7. デプロイメントガイド

### 7.1 L2 Standard 構成例

```yaml
# docker-compose.yml (開発/小規模向け)

services:
  chinju-sidecar:
    image: chinju/sidecar:latest
    environment:
      - CHINJU_SECURITY_LEVEL=L2
      - CHINJU_HSM_TYPE=software_tpm
    devices:
      - /dev/tpm0:/dev/tpm0
    volumes:
      - tpm-state:/var/lib/chinju/tpm

  chinju-registry:
    image: chinju/registry:latest
    environment:
      - CHINJU_SECURITY_LEVEL=L2
```

### 7.2 L3 Enterprise 構成例

```yaml
# L3 構成では実ハードウェアが必要

hardware_requirements:
  hsm:
    type: YubiHSM2
    count: 2  # 冗長構成
    config:
      pkcs11_module: /usr/lib/pkcs11/yubihsm_pkcs11.so

  tee:
    type: intel_sgx
    enclave_size: 256MB

  random:
    type: rdrand  # Intel TRNG
    fallback: /dev/hwrng
```

---

## 8. 参照文書

- [CHINJU-000: Trust Model](./CHINJU-000-TRUST_MODEL.md)
- [CHINJU-001: Architecture Overview](./CHINJU-001-ARCHITECTURE.md)
- [sample/hardware/REQUIREMENTS.md](../sample/hardware/REQUIREMENTS.md)
- [sample/hardware/CERTIFICATION_GUIDE.md](../sample/hardware/CERTIFICATION_GUIDE.md)

---

## 変更履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 0.1.0 | 2026-01-31 | 初版作成 |
