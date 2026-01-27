# OTP/eFuse Unit - Reference Design

## 概要

不変記憶域（OTP/eFuse）は、CHINJUの信頼の根幹を担う。
一度書き込まれたデータは**物理的に変更不可能**であり、AIシステムによる改ざんを不可能にする。

### 対応トレイト

```rust
pub trait ImmutableStorage: Send + Sync {
    fn read_root_key(&self) -> Result<PublicKey, Self::Error>;
    fn read_counter(&self) -> Result<u64, Self::Error>;
    fn increment_counter(&self) -> Result<u64, Self::Error>;
    fn read_threshold_record(&self) -> Result<SignedThresholdRecord, Self::Error>;
    fn is_hardware_backed(&self) -> bool;
}
```

---

## 1. 設計パターン

### Pattern A: MCU内蔵OTP（低コスト）

**用途**: エッジデバイス、IoT、プロトタイプ
**レベル**: Level 1-2

```
┌─────────────────────────────────────────────────────┐
│  MCU (STM32L5 / NXP LPC55S / Nordic nRF5340)       │
│  ┌───────────────────────────────────────────────┐  │
│  │  Flash Memory                                 │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  OTP Area (512B - 4KB)                  │  │  │
│  │  │  ┌───────────────────────────────────┐  │  │  │
│  │  │  │ Root Key (32B)                    │  │  │  │
│  │  │  │ Thresholds (16B)                  │  │  │  │
│  │  │  │ Counter (8B × N slots)            │  │  │  │
│  │  │  │ Reserved                          │  │  │  │
│  │  │  └───────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  TrustZone Secure World                       │  │
│  │  (OTPアクセス制御)                            │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**BOM例**:

| 部品 | 型番 | ベンダー | 単価 |
|------|------|---------|------|
| MCU | STM32L562VET6 | ST | $8 |
| 代替MCU | LPC55S69JBD100 | NXP | $7 |
| 代替MCU | nRF5340 | Nordic | $6 |

**メモリマップ**:

```
0x0000 - 0x001F: Root Public Key (Ed25519, 32 bytes)
0x0020 - 0x0027: θ_cert (f64, 8 bytes)
0x0028 - 0x002F: θ_revoke (f64, 8 bytes)
0x0030 - 0x0037: Version (u64, 8 bytes)
0x0038 - 0x0077: Signature (64 bytes)
0x0078 - 0x007F: Counter Slot 0 (u64)
0x0080 - 0x0087: Counter Slot 1 (u64)
...
0x01F8 - 0x01FF: Counter Slot 47 (u64)
```

**単調増加カウンタの実装**:

```rust
/// カウンタスロット方式
/// 各スロットは一度だけ書き込み可能
/// カウンタ値 = 使用済みスロット数
pub fn increment_counter(&mut self) -> Result<u64, OtpError> {
    // 未使用スロットを探す
    for i in 0..MAX_SLOTS {
        if !self.is_slot_used(i) {
            // スロットを焼く（不可逆）
            self.burn_slot(i)?;
            return Ok(i as u64 + 1);
        }
    }
    Err(OtpError::CounterExhausted)
}
```

---

### Pattern B: Secure Element連携（中コスト）

**用途**: 企業向け、高セキュリティIoT
**レベル**: Level 2-3

```
┌─────────────────────────────────────────────────────┐
│  Host MCU                                           │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Software Layer                        │  │
│  │  (Rust)                                       │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ I2C                      │
└──────────────────────────│──────────────────────────┘
                           │
┌──────────────────────────│──────────────────────────┐
│  Secure Element          │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  ATECC608B / SE050                            │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  OTP Zone (72-512 bytes)                │  │  │
│  │  │  • Root Key (Slot 0)                    │  │  │
│  │  │  • Thresholds (Slot 1)                  │  │  │
│  │  │  • Counter (Monotonic Counter)          │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Crypto Engine                          │  │  │
│  │  │  • ECDSA Sign/Verify                    │  │  │
│  │  │  • SHA-256                              │  │  │
│  │  │  • AES-128                              │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**BOM例**:

| 部品 | 型番 | ベンダー | 単価 |
|------|------|---------|------|
| SE | ATECC608B-MAHDA | Microchip | $0.80 |
| 代替SE | SE050C2HQ1 | NXP | $2.50 |
| 代替SE | OPTIGA Trust M | Infineon | $1.50 |

**Rustバインディング例**:

```rust
use cryptoauthlib::{AtcaClient, AtcaSlot};

pub struct AteccImmutableStorage {
    client: AtcaClient,
}

impl ImmutableStorage for AteccImmutableStorage {
    type Error = AtcaError;

    fn read_root_key(&self) -> Result<PublicKey, Self::Error> {
        // Slot 0 から公開鍵を読み取り
        let pubkey = self.client.get_pubkey(AtcaSlot::Slot0)?;
        Ok(PublicKey::from_bytes(&pubkey))
    }

    fn read_counter(&self) -> Result<u64, Self::Error> {
        // ハードウェア単調増加カウンタを読み取り
        let count = self.client.counter_read(0)?;
        Ok(count as u64)
    }

    fn increment_counter(&self) -> Result<u64, Self::Error> {
        // カウンタをインクリメント（不可逆）
        let new_count = self.client.counter_increment(0)?;
        Ok(new_count as u64)
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern C: 専用OTPチップ（高信頼性）

**用途**: 金融、医療、インフラ
**レベル**: Level 3-4

```
┌─────────────────────────────────────────────────────┐
│  Host System                                        │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Software Layer                        │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ SPI (暗号化)             │
└──────────────────────────│──────────────────────────┘
                           │
┌──────────────────────────│──────────────────────────┐
│  OTP Security Module     │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  Secure MCU (e.g., MAX32520)                  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  eFuse Array (8Kb)                      │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Tamper Detection                       │  │  │
│  │  │  • Voltage monitor                      │  │  │
│  │  │  • Temperature monitor                  │  │  │
│  │  │  • Mesh detection                       │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │  Encapsulation                                │  │
│  │  • Epoxy potting                             │  │
│  │  • Mesh layer                                │  │
│  │  • Light/probe detection                     │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**BOM例**:

| 部品 | 型番 | ベンダー | 単価 |
|------|------|---------|------|
| Secure MCU | MAX32520 | Analog Devices | $15 |
| 代替 | SAMA5D2 | Microchip | $12 |
| ポッティング材 | - | 各社 | $5 |

---

## 2. セキュリティ考慮事項

### 2-1. 書き込み保護

```
Phase 1: 製造時書き込み
  ↓ eFuse焼付
Phase 2: ロック
  ↓ ロックビット設定
Phase 3: 運用（読み取りのみ）
```

**要件**:
- 製造後のロックは**不可逆**であること
- ロック後は書き込みコマンド自体を無効化

### 2-2. サイドチャネル対策

| 攻撃 | 対策 |
|------|------|
| 電力解析 | 定電力回路、ダミー操作 |
| タイミング解析 | 定時間実行 |
| 電磁解析 | シールド、ノイズ注入 |
| 光学プローブ | メッシュ層、光検出 |

### 2-3. フェイルセーフ

```rust
// カウンタ枯渇時の挙動
impl ImmutableStorage for SecureOtp {
    fn increment_counter(&self) -> Result<u64, Self::Error> {
        if self.remaining_slots() == 0 {
            // カウンタ枯渇 = システム停止
            // 新しいハードウェアへの移行が必要
            Err(OtpError::CounterExhausted {
                message: "Counter exhausted. Hardware replacement required.",
                last_value: self.current_count(),
            })
        } else {
            self.do_increment()
        }
    }
}
```

---

## 3. 製造プロセス

### 3-1. 鍵セレモニー

```
1. 安全な環境（SCIF/Faraday cage）で鍵生成
2. ルート秘密鍵をHSMに保管
3. ルート公開鍵をOTPに書き込み
4. 閾値（θ_cert, θ_revoke）を書き込み
5. OTPをロック
6. 検証（読み取り確認）
7. 証明書発行
```

### 3-2. 品質管理

| 検査項目 | 基準 |
|---------|------|
| 書き込み検証 | 全ビット一致 |
| 読み取り検証 | 1000回連続一致 |
| 温度サイクル | -40°C〜+85°C, 100回 |
| ESD耐性 | ±8kV (HBM) |

---

## 4. ファームウェア構造

```
otp_driver/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── traits.rs          # ImmutableStorage実装
│   ├── drivers/
│   │   ├── stm32l5.rs      # STM32L5 OTP
│   │   ├── atecc608b.rs    # ATECC608B
│   │   └── se050.rs        # NXP SE050
│   ├── memory_map.rs       # メモリレイアウト
│   └── ceremony.rs         # 鍵セレモニーユーティリティ
└── examples/
    ├── read_root_key.rs
    └── increment_counter.rs
```

---

## 5. ベンダー多様性

| 要件 | プライマリ | セカンダリ | ターシャリ |
|------|-----------|-----------|-----------|
| MCU | STM32L5 | NXP LPC55S | Nordic nRF5340 |
| SE | ATECC608B | SE050 | OPTIGA Trust |

**切り替え設定**:

```toml
[hardware.otp]
driver = "atecc608b"  # or "stm32l5", "se050"

[hardware.otp.atecc608b]
i2c_bus = "/dev/i2c-1"
address = 0x6A

[hardware.otp.stm32l5]
otp_base = 0x1FFF7000
otp_size = 1024
```

---

## 参照文書

- `chinju/traits/hardware_abstraction.md` - トレイト定義
- `chinju/hardware/REQUIREMENTS.md` - 要件
- Microchip AN-2042 - ATECC608B Configuration
- ST RM0438 - STM32L5 OTP Programming
