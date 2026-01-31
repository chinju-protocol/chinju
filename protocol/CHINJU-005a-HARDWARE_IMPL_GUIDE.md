# CHINJU-005a: Hardware Implementation Guide（ハードウェア実装ガイド）

**Status:** Active
**Version:** 1.0.0
**Date:** 2026-01-31
**Parent:** [CHINJU-005-HARDWARE.md](./CHINJU-005-HARDWARE.md)

---

## 1. 概要

本文書は、CHINJUプロトコルのハードウェア関連機能を**実際に実装・テスト**するためのガイドです。
概念的な要件は [CHINJU-005-HARDWARE.md](./CHINJU-005-HARDWARE.md) を参照してください。

### 1.1 対象読者

- ハードウェアエンジニア
- 組み込みシステム開発者
- セキュリティエンジニア
- DevOps エンジニア（CI/CD 環境構築）

### 1.2 前提知識

- Rust プログラミング
- TPM 2.0 の基本概念
- Linux デバイスドライバの基礎
- Docker / Docker Compose

---

## 2. 実装状況一覧

### 2.1 セキュリティレベル

| レベル | 名称 | 説明 | 用途 |
|:---:|:---|:---|:---|
| L0 | Mock | ソフトウェアモック | 開発・単体テスト |
| L1 | Software | SoftHSM / swtpm | CI/CD・統合テスト |
| L2 | Hardware Standard | TPM 2.0 | 本番（標準） |
| L3 | Hardware Premium | YubiHSM / TEE | 本番（高セキュリティ） |

### 2.2 コンポーネント別実装状況

| コンポーネント | L0 Mock | L1 Software | L2 HW Standard | L3 HW Premium |
|:---|:---:|:---:|:---:|:---:|
| **HSM（署名・鍵管理）** | `MockHsm` ✅ | `SoftHsm` ✅ | `TpmHsm` ✅ | YubiHSM ⏳ |
| **乱数生成** | `MockRandom` ✅ | SoftHSM ✅ | TPM RNG ✅ | QRNG ⏳ |
| **OTP（不変ストレージ）** | `MockOtp` ✅ | - | TPM NV ✅ | eFuse ⏳ |
| **Dead Man's Switch** | `SoftDMS` ✅ | `TpmDMS` + swtpm ✅ | `TpmDMS` ✅ | + センサー ⏳ |
| **PCR（改ざん検知）** | - | swtpm ✅ | TPM PCR ✅ | - |
| **Seal/Unseal** | - | swtpm ✅ | TPM Seal ✅ | TEE Seal ⏳ |
| **構成証明（Quote）** | - | swtpm Quote ✅ | TPM Quote ✅ | TEE Attest ⏳ |
| **閾値署名** | - | FROST ✅ | FROST + TPM ⏳ | FROST + HSM ⏳ |

### 2.3 ソースコード対応表

| コンポーネント | トレイト定義 | Mock 実装 | Software 実装 | Hardware 実装 |
|:---|:---|:---|:---|:---|
| HSM | `traits.rs:HardwareSecurityModule` | `mock/hsm.rs` | `softhsm/hsm.rs` | `tpm/hsm.rs` |
| OTP | `traits.rs:ImmutableStorage` | `mock/otp.rs` | - | TPM NV Memory |
| Random | `traits.rs:RandomSource` | `mock/random.rs` | SoftHSM RNG | `TpmHsm::generate()` |
| DMS | `dead_mans_switch.rs:DeadMansSwitch` | `SoftDeadMansSwitch` | `TpmDMS` + swtpm | `tpm/dead_mans_switch.rs` |
| TEE | `traits.rs:SecureExecution` | - | - | ⏳ 待機 |

---

## 3. 開発環境セットアップ

### 3.1 必要なツール

```bash
# Rust ツールチェイン
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Protocol Buffers
# macOS
brew install protobuf

# Ubuntu/Debian
apt-get install protobuf-compiler

# Docker (必須: TPM テスト用)
# https://docs.docker.com/get-docker/
```

### 3.2 swtpm（ソフトウェア TPM）セットアップ

#### 方法1: Docker Compose（推奨）

```bash
cd chinju-protocol

# swtpm サービスを起動
docker-compose up -d swtpm

# 確認
docker-compose ps
# NAME      COMMAND                  SERVICE   STATUS
# swtpm     "/usr/bin/swtpm sock…"   swtpm     Up

# ログ確認
docker-compose logs swtpm
```

#### 方法2: ネイティブインストール

**macOS:**
```bash
brew install swtpm

# 状態保存ディレクトリを作成
mkdir -p /tmp/tpm

# swtpm を起動（ソケットモード）
swtpm socket --tpmstate dir=/tmp/tpm \
    --ctrl type=tcp,port=2322 \
    --server type=tcp,port=2321 \
    --flags startup-clear
```

**Linux (Ubuntu/Debian):**
```bash
apt-get install swtpm swtpm-tools

mkdir -p /tmp/tpm
swtpm socket --tpmstate dir=/tmp/tpm \
    --ctrl type=tcp,port=2322 \
    --server type=tcp,port=2321 \
    --flags startup-clear
```

### 3.3 環境変数

```bash
# .env ファイルまたはシェルで設定
export TPM_INTERFACE=socket      # socket / device / tabrmd
export TPM_HOST=localhost        # Docker の場合は swtpm
export TPM_PORT=2321             # TPM コマンドポート
export CHINJU_DMS_USE_TPM=true   # TPM Dead Man's Switch 有効化
```

### 3.4 TPM 統合テスト実行

```bash
# 推奨: スクリプトを使用
./scripts/test-tpm.sh

# または手動で
docker-compose -f docker-compose.yml -f docker-compose.test.yml up chinju-tpm-test

# デバッグ用シェル（コンテナ内で作業）
./scripts/test-tpm.sh shell

# クリーンアップ
./scripts/test-tpm.sh clean
```

### 3.5 Feature Flags

`chinju-core` の Cargo.toml で feature を有効化:

```toml
[features]
default = ["mock"]
mock = []           # L0: 開発用モック
softhsm = ["cryptoki"]  # L1: SoftHSM (PKCS#11)
frost = ["frost-ed25519"]  # 閾値署名
tpm = ["tss-esapi"]  # L2: TPM 2.0 (Linux のみ)
```

```bash
# ビルド例
cargo build                          # mock のみ
cargo build --features softhsm       # SoftHSM 有効
cargo build --features tpm           # TPM 有効 (Linux)
cargo build --features "softhsm,frost,tpm"  # 全機能
```

---

## 4. Dead Man's Switch 実装詳細

### 4.1 アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                    GatewayService                            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  process_request() ──→ heartbeat() ──→ check_state()   │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│              DeadMansSwitch trait                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ arm() / disarm() / heartbeat() / state()             │   │
│  │ update_environment() / on_emergency()                │   │
│  └──────────────────────────────────────────────────────┘   │
└───────────────┬─────────────────────────┬───────────────────┘
                │                         │
       ┌────────┴────────┐       ┌────────┴────────┐
       ▼                 ▼       ▼                 ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐
│ SoftDMS (L0) │  │ TpmDMS (L1)  │  │ TpmDMS + Sensors (L3)│
│              │  │   + swtpm    │  │                      │
│ メモリのみ   │  │ NV Memory    │  │ NV Memory            │
│              │  │ PCR 拡張     │  │ PCR 拡張             │
│              │  │ Seal/Unseal  │  │ Seal/Unseal          │
│              │  │              │  │ GPIO センサー        │
└──────────────┘  └──────────────┘  └──────────────────────┘
```

### 4.2 TpmDeadMansSwitch 機能

| 機能 | 説明 | TPM コマンド |
|:---|:---|:---|
| **NV Memory 永続化** | ハートビート時刻・状態を保存（再起動後も維持） | `nv_write` / `nv_read` |
| **PCR 拡張** | 状態変更・異常検知を記録（改ざん検知） | `pcr_extend` |
| **Seal/Unseal** | PCR 値に紐付けた暗号化（環境変更で復号不可） | `create` / `unseal` |
| **緊急消去** | NV クリア + PCR 拡張で sealed data を無効化 | `nv_undefine_space` |
| **Quote** | 現在の PCR 値を署名付きで取得（リモート構成証明） | `quote` |

### 4.3 NV Index 配置

```
TPM NV Memory Map:
┌─────────────────────────────────────────┐
│ 0x01800001: Heartbeat Timestamp (8B)    │
│ 0x01800002: Switch State (1B)           │
│ 0x01800003: Environment State (64B)     │
└─────────────────────────────────────────┘

PCR Slots (SHA-256):
┌─────────────────────────────────────────┐
│ PCR 16: Tamper Detection                │
│ PCR 17: State Tracking                  │
└─────────────────────────────────────────┘
```

### 4.4 状態遷移

```
                    arm()
    ┌──────────┐  ─────────►  ┌──────────┐
    │ Disarmed │              │  Armed   │
    └──────────┘  ◄─────────  └────┬─────┘
                    disarm()       │
                                   │ timeout
                                   ▼
                             ┌──────────┐
                             │  Grace   │
                             │  Period  │
                             └────┬─────┘
                                  │ grace_period expired
                                  │ OR anomaly detected
                                  ▼
                             ┌──────────┐
                             │ Triggered│ ──► Emergency Callbacks
                             └──────────┘     NV Clear + PCR Extend
```

---

## 5. 待機項目: センサー連携仕様

### 5.1 概要

Phase 6.4.2「Dead Man's Switch センサー連携」の実装仕様です。

### 5.2 EnvironmentState 構造体

```rust
// chinju-core/src/hardware/dead_mans_switch.rs

pub struct EnvironmentState {
    /// 筐体内温度 (℃)
    pub temperature: Option<f32>,
    /// 加速度 (G)
    pub acceleration: Option<f32>,
    /// エンクロージャ開封検知
    pub enclosure_opened: bool,
    /// ネットワーク接続状態
    pub network_connected: bool,
    /// AC 電源接続状態
    pub on_mains_power: bool,
}
```

### 5.3 推奨センサー

| センサー種別 | 推奨製品 | インターフェース | 精度 |
|:---|:---|:---|:---|
| **温度** | DS18B20 | 1-Wire / GPIO | ±0.5℃ |
| **温度** | BME280 | I2C / SPI | ±1.0℃ |
| **加速度** | ADXL345 | I2C / SPI | ±2g / ±16g |
| **加速度** | MPU-6050 | I2C | ±2g / ±16g |
| **開封検知** | リードスイッチ | GPIO (Pull-up) | - |
| **開封検知** | 光センサー | GPIO (ADC) | - |

### 5.4 GPIO ピン配置（参考: Raspberry Pi）

```
┌─────────────────────────────────────────────────────────────┐
│  Raspberry Pi 4 GPIO Header                                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Pin 1  (3.3V)  ─────── センサー電源                        │
│  Pin 3  (GPIO2/SDA) ─── I2C データ (加速度/温度)            │
│  Pin 5  (GPIO3/SCL) ─── I2C クロック                        │
│  Pin 7  (GPIO4) ──────── 1-Wire データ (DS18B20)           │
│  Pin 11 (GPIO17) ─────── 開封検知 (リードスイッチ)          │
│  Pin 13 (GPIO27) ─────── 緊急停止ボタン                     │
│  Pin 6  (GND) ────────── センサー GND                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.5 実装インターフェース（提案）

```rust
// chinju-core/src/hardware/sensors.rs (未実装)

/// センサー読み取りトレイト
pub trait EnvironmentSensor: Send + Sync {
    /// 現在の環境状態を取得
    fn read(&self) -> Result<EnvironmentState, SensorError>;

    /// センサーの健全性チェック
    fn health_check(&self) -> Result<bool, SensorError>;

    /// キャリブレーション
    fn calibrate(&mut self) -> Result<(), SensorError>;
}

/// GPIO ベースのセンサー実装
pub struct GpioEnvironmentSensor {
    /// I2C バス
    i2c: I2cBus,
    /// 開封検知 GPIO ピン
    tamper_pin: GpioPin,
    /// 緊急停止 GPIO ピン
    emergency_pin: GpioPin,
}

impl EnvironmentSensor for GpioEnvironmentSensor {
    fn read(&self) -> Result<EnvironmentState, SensorError> {
        Ok(EnvironmentState {
            temperature: self.read_temperature()?,
            acceleration: self.read_acceleration()?,
            enclosure_opened: self.tamper_pin.is_high()?,
            network_connected: self.check_network()?,
            on_mains_power: self.check_power()?,
        })
    }
}
```

### 5.6 異常検知閾値

```rust
// DeadMansSwitchConfig のデフォルト値

pub struct DeadMansSwitchConfig {
    /// ハートビート間隔
    pub heartbeat_interval: Duration,     // 30秒
    /// タイムアウト閾値
    pub heartbeat_timeout: Duration,      // 90秒 (3回連続失敗)
    /// グレースピリオド
    pub grace_period: Duration,           // 60秒

    /// 温度異常閾値（下限）
    pub temp_min: f32,                    // 0.0℃
    /// 温度異常閾値（上限）
    pub temp_max: f32,                    // 50.0℃
    /// 振動異常閾値
    pub acceleration_threshold: f32,      // 1.0G
}
```

### 5.7 実装タスク

| タスク | 優先度 | 依存 | 備考 |
|:---|:---:|:---|:---|
| `sensors.rs` モジュール作成 | 高 | なし | トレイト定義 |
| GPIO ライブラリ選定 | 高 | なし | `rppal` or `gpio-cdev` |
| I2C センサー実装 (温度) | 中 | GPIO | BME280 / DS18B20 |
| I2C センサー実装 (加速度) | 中 | GPIO | ADXL345 / MPU-6050 |
| 開封検知実装 | 中 | GPIO | リードスイッチ |
| TpmDMS + センサー統合 | 低 | 上記全て | 最終統合 |

---

## 6. 待機項目: TEE 統合仕様

### 6.1 概要

Phase 6.5「セキュアエンクレーブ」および Phase 6.6.1「TEE リモート構成証明」の実装仕様です。

### 6.2 対応 TEE プラットフォーム

| プラットフォーム | プロバイダ | 対応状況 | 優先度 |
|:---|:---|:---:|:---:|
| **AWS Nitro Enclaves** | AWS | ⏳ 待機 | 高 |
| **Azure Confidential Computing** | Microsoft | ⏳ 待機 | 中 |
| **Intel SGX** | Intel | ⏳ 待機 | 中 |
| **ARM TrustZone** | ARM | ⏳ 待機 | 低 |
| **NVIDIA Confidential Computing** | NVIDIA | ⏳ 待機 | 将来 |

### 6.3 SecureExecution トレイト（既存）

```rust
// chinju-core/src/hardware/traits.rs

/// 信頼実行環境トレイト
pub trait SecureExecution: TrustRoot {
    /// エンクレーブ内で関数を実行
    fn execute_secure<F, R>(&self, f: F) -> Result<R, HardwareError>
    where
        F: FnOnce() -> R + Send,
        R: Send;

    /// データを封印（エンクレーブ鍵で暗号化）
    fn seal_data(&self, data: &[u8]) -> Result<Vec<u8>, HardwareError>;

    /// データを開封（エンクレーブ鍵で復号）
    fn unseal_data(&self, sealed: &[u8]) -> Result<Vec<u8>, HardwareError>;

    /// リモート構成証明レポートを取得
    fn remote_attestation(&self, challenge: &[u8]) -> Result<Vec<u8>, HardwareError>;
}
```

### 6.4 AWS Nitro Enclaves 実装（提案）

```rust
// chinju-core/src/hardware/nitro/mod.rs (未実装)

use aws_nitro_enclaves_sdk::*;

pub struct NitroEnclave {
    enclave_id: String,
    vsock_port: u32,
}

impl SecureExecution for NitroEnclave {
    fn execute_secure<F, R>(&self, f: F) -> Result<R, HardwareError>
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        // Nitro Enclave との vsock 通信
        // シリアライズ → 送信 → 実行 → 受信 → デシリアライズ
        todo!()
    }

    fn remote_attestation(&self, challenge: &[u8]) -> Result<Vec<u8>, HardwareError> {
        // NSM (Nitro Security Module) から attestation document を取得
        let nsm = NsmConnection::new()?;
        let doc = nsm.get_attestation_document(
            Some(challenge),  // user_data
            None,             // nonce
            None,             // public_key
        )?;
        Ok(doc.to_cbor()?)
    }
}
```

### 6.5 実装タスク

| タスク | 優先度 | 依存 | 備考 |
|:---|:---:|:---|:---|
| Nitro SDK 調査 | 高 | なし | `aws-nitro-enclaves-sdk-rs` |
| `nitro/` モジュール作成 | 高 | SDK | トレイト実装 |
| Attestation 検証ロジック | 中 | モジュール | AWS 証明書チェーン |
| sidecar Nitro 対応 | 中 | モジュール | feature flag 追加 |
| Terraform / CDK | 低 | 上記 | インフラ定義 |

---

## 7. 待機項目: YubiHSM 統合仕様

### 7.1 概要

L3 セキュリティレベル向けの YubiHSM 2 統合仕様です。

### 7.2 製品仕様

| 項目 | 値 |
|:---|:---|
| モデル | YubiHSM 2 |
| 認証 | FIPS 140-2 Level 3 |
| インターフェース | USB / HTTP (yubihsm-connector) |
| 対応アルゴリズム | RSA (2048-4096), ECDSA (P-256/384), Ed25519 |
| 鍵スロット | 最大 256 |

### 7.3 接続構成

```
┌──────────────┐     USB      ┌──────────────┐
│   YubiHSM 2  │─────────────►│    Server    │
└──────────────┘              └──────┬───────┘
                                     │
                              ┌──────┴───────┐
                              │ yubihsm-     │
                              │ connector    │
                              │ :12345       │
                              └──────┬───────┘
                                     │ HTTP
                              ┌──────┴───────┐
                              │ chinju-core  │
                              │ (libyubihsm) │
                              └──────────────┘
```

### 7.4 実装インターフェース（提案）

```rust
// chinju-core/src/hardware/yubihsm/mod.rs (未実装)

use yubihsm::{Client, Connector, Credentials};

pub struct YubiHsm {
    client: Client,
}

impl YubiHsm {
    pub fn connect(connector_url: &str, auth_key_id: u16, password: &str) -> Result<Self, HardwareError> {
        let connector = Connector::http(connector_url)?;
        let credentials = Credentials::from_password(auth_key_id, password.as_bytes());
        let client = Client::open(connector, credentials, true)?;
        Ok(Self { client })
    }
}

impl HardwareSecurityModule for YubiHsm {
    fn generate_key_pair(&self, algorithm: KeyAlgorithm, label: &str) -> Result<KeyHandle, HardwareError> {
        let key_id = self.client.generate_asymmetric_key(
            0,  // key_id (0 = auto-assign)
            label.to_string(),
            0xFFFF,  // domains (all)
            yubihsm::object::Capability::SIGN_ECDSA | yubihsm::object::Capability::EXPORTABLE_UNDER_WRAP,
            yubihsm::asymmetric::Algorithm::EcP256,
        )?;
        Ok(KeyHandle::new(key_id.to_string()))
    }

    // ... 他のメソッド
}
```

---

## 8. テスト戦略

### 8.1 テストレベル

| レベル | 環境 | テスト内容 |
|:---|:---|:---|
| L0 | ローカル | Mock を使用した単体テスト |
| L1 | Docker | swtpm / SoftHSM を使用した統合テスト |
| L2 | 実機 | TPM 2.0 チップを使用したテスト |
| L3 | 実機 | YubiHSM / TEE を使用したテスト |

### 8.2 CI/CD パイプライン

```yaml
# .github/workflows/hardware-tests.yml

name: Hardware Tests

on: [push, pull_request]

jobs:
  mock-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run mock tests
        run: cargo test

  tpm-tests:
    runs-on: ubuntu-latest
    services:
      swtpm:
        image: ghcr.io/tpm2-software/swtpm:latest
        ports:
          - 2321:2321
          - 2322:2322
    steps:
      - uses: actions/checkout@v4
      - name: Run TPM tests
        run: cargo test --features tpm -- --ignored
        env:
          TPM_INTERFACE: socket
          TPM_HOST: localhost
          TPM_PORT: 2321
```

### 8.3 テスト実行コマンド

```bash
# L0: Mock テスト
cargo test

# L1: swtpm テスト
./scripts/test-tpm.sh

# L1: SoftHSM テスト
cargo test --features softhsm

# L2: 実機 TPM テスト (Linux + TPM 2.0)
TPM_INTERFACE=device TPM_DEVICE=/dev/tpm0 cargo test --features tpm -- --ignored
```

---

## 9. トラブルシューティング

### 9.1 swtpm 接続エラー

```
Error: TpmError::ConnectionFailed
```

**原因:** swtpm が起動していない、またはポートが間違っている

**解決:**
```bash
# swtpm が起動しているか確認
docker-compose ps

# ポートを確認
netstat -an | grep 2321

# 再起動
docker-compose restart swtpm
```

### 9.2 NV Index エラー

```
Error: NV index already defined
```

**原因:** 前回のテストで NV index が残っている

**解決:**
```bash
# swtpm 状態をクリア
./scripts/test-tpm.sh clean
docker-compose up -d swtpm
```

### 9.3 TPM feature が Linux でないとビルドできない

```
error: tpm feature requires Linux
```

**原因:** `tss-esapi` クレートは Linux のみサポート

**解決:**
```bash
# Docker 内でビルド
docker run -v $(pwd):/work -w /work rust:latest cargo build --features tpm
```

---

## 10. 参照文書

### プロジェクト内

- [CHINJU-005-HARDWARE.md](./CHINJU-005-HARDWARE.md) - ハードウェア要件定義
- [plans.md](../.claude/plans.md) - 実装計画書
- [LLM_CONTEXT.md](../.claude/LLM_CONTEXT.md) - 開発者ガイド

### 外部リソース

- [TPM 2.0 Specification](https://trustedcomputinggroup.org/resource/tpm-library-specification/)
- [tss-esapi (Rust TPM library)](https://docs.rs/tss-esapi/)
- [swtpm (Software TPM)](https://github.com/stefanberger/swtpm)
- [YubiHSM 2 Documentation](https://developers.yubico.com/YubiHSM2/)
- [AWS Nitro Enclaves](https://docs.aws.amazon.com/enclaves/latest/user/)

---

## 変更履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 1.0.0 | 2026-01-31 | 初版作成 |
